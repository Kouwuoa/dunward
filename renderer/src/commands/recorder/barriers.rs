//! Pipeline memory barriers, layout transitions, and queue ownership transfers.
//!
//! Implements [`CommandRecorder::insert_texture_memory_barrier`] and its high-level helpers
//! ([`transition_texture`], [`sync_texture`], [`sync_texture_same_access`],
//! [`release_texture_to_queue`], [`prepare_texture_for_presentation`]).

use std::sync::Arc;

use ash::vk;

use super::{CommandRecorder, Recording};
use crate::gpu::queue::Queue;
use crate::resources::texture::{Texture, TextureAccess, TextureQueueState};

impl CommandRecorder<Recording> {
    /// Records a Vulkan pipeline barrier (`vkCmdPipelineBarrier2`) for an image subresource,
    /// managing layout transitions, pipeline stage execution dependencies, memory access hazards,
    /// and queue family ownership transfers (QFOT).
    ///
    /// This method automatically uses the texture's internally tracked state (`layout`, `stage_mask`,
    /// `access_mask`, and `queue_state`) to compute source barrier masks, eliminating the need to
    /// manually track what stage previously touched the texture.
    ///
    /// # Arguments
    ///
    /// * `texture` - A mutable reference to the [`Texture`] being transitioned. Its internal state
    ///   machine (`layout`, `access_state`, and `queue_state`) will be updated after
    ///   the barrier is recorded.
    ///
    /// * `dst_layout` - Optional target [`vk::ImageLayout`] for the image.
    ///   * If `Some(layout)`: Transitions the image from its current layout into the new layout.
    ///   * If `None`: Keeps the texture in its current layout, recording a pure execution/memory
    ///     barrier (cache flush and stage sync between passes using the same layout).
    ///
    /// * `dst_access` - Optional [`TextureAccess`] specifying the downstream `stage_mask` and `access_mask`
    ///   that will consume or write to the texture next on the current queue.
    ///   * If `Some(access)`: Synchronizes against previous operations and establishes an execution dependency
    ///     for the incoming stage. If the access is a write, it resets the tracked stage/access state; if read-only,
    ///     it accumulates (`|=`) to prevent Write-After-Read (WAR) hazards.
    ///   * If `None`: Defaults to `(NONE, NONE)`. Appropriate when releasing a texture to another queue family
    ///     or preparing a swapchain image for presentation.
    ///
    /// * `dst_queue` - Destination [`Queue`] for a Queue Family Ownership Transfer (QFOT).
    ///   * `Some(target_queue)`: Initiates a **Release operation** transferring ownership from this recorder's
    ///     queue to `target_queue`.
    ///   * `None`: Performs an **Acquire operation** if the texture was previously released by another queue
    ///     or does nothing if the texture is already owned by this recorder's queue.
    ///
    /// # Panics
    ///
    /// * If `dst_layout` is `None` (or `Some(UNDEFINED)`) when the texture is currently in [`vk::ImageLayout::UNDEFINED`].
    /// * If `dst_access` contains `vk::PipelineStageFlags2::NONE` or `vk::AccessFlags2::NONE` (pass `None` instead).
    /// * If attempting to release a texture that is owned by a different queue family.
    /// * If attempting to release a texture that is already in a `Transferring` state without having been acquired first (Double Release).
    /// * If attempting to acquire a texture whose pending transfer was directed to a different queue family.
    /// * If attempting to use a texture owned by another queue without an ownership transfer.
    fn insert_texture_memory_barrier(
        &self,
        texture: &mut Texture,
        dst_layout: Option<vk::ImageLayout>,
        dst_access: Option<TextureAccess>,
        dst_queue: Option<Arc<Queue>>,
    ) {
        // Validate dst_layout
        if let Some(layout) = dst_layout {
            assert_ne!(
                layout,
                vk::ImageLayout::UNDEFINED,
                "Invalid argument: dst_layout was `Some(UNDEFINED)`. Pass `None` if no layout is required."
            );
        } else {
            assert_ne!(
                texture.layout,
                vk::ImageLayout::UNDEFINED,
                "Invalid argument: dst_layout was `None` and texture is currently in UNDEFINED layout. Pass `Some(layout)` to specify a valid layout."
            );
        }
        // If no layout is provided, use the current layout of the texture, indicating that the layout will not be changing
        let dst_layout = dst_layout.unwrap_or(texture.layout);
        // Disallow UNDEFINED dst_layout
        assert_ne!(
            dst_layout,
            vk::ImageLayout::UNDEFINED,
            "Invalid argument: dst_layout must be a valid layout and not UNDEFINED"
        );

        // Validate dst_access
        if let Some(access) = dst_access {
            assert!(
                access.stage_mask != vk::PipelineStageFlags2::NONE && !access.stage_mask.is_empty(),
                "Invalid argument: dst_stage_mask was `Some(NONE)`. Pass `None` if no stage mask is required."
            );
            assert!(
                access.access_mask != vk::AccessFlags2::NONE && !access.access_mask.is_empty(),
                "Invalid argument: dst_access_mask was `Some(NONE)`. Pass `None` if no access mask is required."
            );
        }
        let dst_stage_mask = if let Some(access) = dst_access {
            access.stage_mask
        } else {
            vk::PipelineStageFlags2::NONE
        };
        let dst_access_mask = if let Some(access) = dst_access {
            access.access_mask
        } else {
            vk::AccessFlags2::NONE
        };
        let is_write = (dst_access_mask
            & (vk::AccessFlags2::SHADER_STORAGE_WRITE
            | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE
            | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE
            | vk::AccessFlags2::TRANSFER_WRITE))
            != vk::AccessFlags2::NONE;

        let recorder_queue_family_index = self.queue.family.index;
        let mut queue_state_to_apply = None;
        let (
            src_queue_family_index,
            dst_queue_family_index,
            src_stage_mask,
            src_access_mask,
            dst_stage_mask,
            dst_access_mask,
        ) = match (&texture.queue_state, dst_queue) {
            // Case 0: First use of a brand-new texture -> Claim ownership with this recorder
            (TextureQueueState::Uninitialized, dst_queue) => {
                if let Some(dst_queue) = dst_queue {
                    // If the destination queue is different from the recorder queue, begin transferring ownership
                    if dst_queue.family.index != recorder_queue_family_index {
                        queue_state_to_apply = Some(TextureQueueState::Transferring {
                            src_queue: self.queue.clone(),
                            dst_queue: dst_queue.clone(),
                        });
                        (
                            self.queue.family.index,
                            dst_queue.family.index,
                            vk::PipelineStageFlags2::NONE,
                            vk::AccessFlags2::NONE,
                            vk::PipelineStageFlags2::NONE,
                            vk::AccessFlags2::NONE,
                        )
                    // If the destination queue is the same as the recorder queue, assign initial ownership of this texture to this queue
                    } else {
                        queue_state_to_apply = Some(TextureQueueState::Owned {
                            queue: self.queue.clone(),
                        });
                        (
                            vk::QUEUE_FAMILY_IGNORED,
                            vk::QUEUE_FAMILY_IGNORED,
                            vk::PipelineStageFlags2::NONE,
                            vk::AccessFlags2::NONE,
                            dst_stage_mask,
                            dst_access_mask,
                        )
                    }
                // Assign initial ownership of this texture to this queue
                } else {
                    queue_state_to_apply = Some(TextureQueueState::Owned {
                        queue: self.queue.clone(),
                    });
                    (
                        vk::QUEUE_FAMILY_IGNORED,
                        vk::QUEUE_FAMILY_IGNORED,
                        vk::PipelineStageFlags2::NONE,
                        vk::AccessFlags2::NONE,
                        dst_stage_mask,
                        dst_access_mask,
                    )
                }
            }

            // Case 1: RELEASE to another queue
            (TextureQueueState::Owned { queue }, Some(dst_queue)) => {
                assert_eq!(
                    queue.family.index, recorder_queue_family_index,
                    "Cannot release texture from Queue {:?} because texture is currently owned by Queue {:?}",
                    recorder_queue_family_index, queue.family.index
                );

                if dst_queue.family.index != recorder_queue_family_index {
                    queue_state_to_apply = Some(TextureQueueState::Transferring {
                        src_queue: queue.clone(),
                        dst_queue: dst_queue.clone(),
                    });
                    (
                        queue.family.index,
                        dst_queue.family.index,
                        texture.access_state.stage_mask,
                        texture.access_state.access_mask,
                        vk::PipelineStageFlags2::NONE,
                        vk::AccessFlags2::NONE,
                    )
                } else {
                    // Ignore case where the texture is already owned by the recorder queue
                    (
                        vk::QUEUE_FAMILY_IGNORED,
                        vk::QUEUE_FAMILY_IGNORED,
                        texture.access_state.stage_mask,
                        texture.access_state.access_mask,
                        dst_stage_mask,
                        dst_access_mask,
                    )
                }
            }

            // Case 2: ACQUIRE from a pending transfer
            (
                TextureQueueState::Transferring {
                    src_queue,
                    dst_queue,
                },
                None,
            ) => {
                assert_eq!(
                    dst_queue.family.index, recorder_queue_family_index,
                    "Queue {:?} attempted to acquire texture, but pending transfer was directed to Queue {:?}",
                    recorder_queue_family_index, dst_queue.family.index
                );

                queue_state_to_apply = Some(TextureQueueState::Owned {
                    queue: dst_queue.clone(),
                });
                (
                    src_queue.family.index,
                    dst_queue.family.index,
                    vk::PipelineStageFlags2::NONE,
                    vk::AccessFlags2::NONE,
                    dst_stage_mask,
                    dst_access_mask,
                )
            }

            // Case 3: Ignore same-queue transition
            (TextureQueueState::Owned { queue }, None) => {
                assert_eq!(
                    queue.family.index, recorder_queue_family_index,
                    "Queue {:?} attempted to use texture, but texture is owned by Queue {:?}",
                    recorder_queue_family_index, queue.family.index
                );

                if dst_layout != vk::ImageLayout::PRESENT_SRC_KHR {
                    assert!(
                        dst_access.is_some(),
                        "dst_access must be provided for internal layout transitions unless transitioning to PRESENT_SRC_KHR."
                    )
                }

                (
                    vk::QUEUE_FAMILY_IGNORED,
                    vk::QUEUE_FAMILY_IGNORED,
                    texture.access_state.stage_mask,
                    texture.access_state.access_mask,
                    dst_stage_mask,
                    dst_access_mask,
                )
            }

            // Case 4: ERROR when attempting to release a texture that is already pending a transfer
            (
                TextureQueueState::Transferring {
                    src_queue,
                    dst_queue,
                },
                Some(_),
            ) => {
                panic!(
                    "Double queue release detected! Texture is already transferring from Queue {:?} to Queue {:?} and has not been acquired.",
                    src_queue.family.index, dst_queue.family.index
                );
            }
        };

        let image_barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(src_stage_mask)
            .src_access_mask(src_access_mask)
            .dst_stage_mask(dst_stage_mask)
            .dst_access_mask(dst_access_mask)
            .old_layout(texture.layout)
            .new_layout(dst_layout)
            .src_queue_family_index(src_queue_family_index)
            .dst_queue_family_index(dst_queue_family_index)
            .image(texture.image)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: texture.aspect,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            });

        let dep_info = vk::DependencyInfo::default()
            .image_memory_barriers(std::slice::from_ref(&image_barrier));
        unsafe {
            self.device
                .cmd_pipeline_barrier2(self.command_buffer, &dep_info);
        }

        // Update internal texture tracking
        texture.layout = dst_layout;
        if is_write {
            // Overwrite to reset the masks to only the writing stage because subsequent readers only need for that single write to complete
            texture.access_state.stage_mask = dst_stage_mask;
            texture.access_state.access_mask = dst_access_mask;
        } else {
            // Accumulate all stages currently reading the texture so the next write knows to wait for all previous reads to complete
            texture.access_state.stage_mask |= dst_stage_mask;
            texture.access_state.access_mask |= dst_access_mask;
        }
        if let Some(queue_state_to_apply) = queue_state_to_apply {
            texture.queue_state = queue_state_to_apply;
        }
    }

    /// Standard same-queue transition: changes layout and synchronizes for the next access
    #[inline]
    pub fn transition_texture(
        &self,
        texture: &mut Texture,
        dst_layout: vk::ImageLayout,
        dst_access: TextureAccess,
    ) {
        self.insert_texture_memory_barrier(texture, Some(dst_layout), Some(dst_access), None);
    }

    /// Synchronizes consecutive operations on a texture within the **same pipeline stage and access mode**,
    /// keeping its layout and access state unchanged.
    ///
    /// This method inserts an execution and memory barrier where both the source and destination
    /// synchronization scopes match the texture's current [`TextureAccess`] state (`src == dst`).
    /// It ensures that all memory writes from previous dispatches or draws in this stage are flushed
    /// and made visible before subsequent work in the same stage begins.
    ///
    /// # When to Use
    ///
    /// * **Consecutive Compute Dispatches**: Pass 1 writes to a storage image, and Pass 2
    ///   reads/writes to that same storage image in `COMPUTE_SHADER` stage.
    /// * **Iterative Algorithms**: Multi-pass compute algorithms (e.g. blur passes, particle simulation steps,
    ///   or raymarching iterations) executing sequentially on the same texture.
    ///
    /// # Important
    ///
    /// Do **not** use this method if the pipeline stage or access mode is changing (e.g. transitioning
    /// from a clear operation in the `TRANSFER` stage to a `COMPUTE_SHADER`). In those cases, use
    /// [`sync_texture`](Self::sync_texture) to explicitly specify the incoming [`TextureAccess`].
    #[inline]
    pub fn sync_texture_same_access(&self, texture: &mut Texture) {
        let dst_access = texture.access_state;
        self.insert_texture_memory_barrier(texture, None, Some(dst_access), None);
    }

    /// Synchronizes access to a texture for a **new pipeline stage or access mode** on the same queue,
    /// without changing its image layout.
    ///
    /// This method inserts a pipeline barrier (`vkCmdPipelineBarrier2`) that connects the texture's
    /// previous operation (`src_stage` and `src_access` from `texture.access_state`) to the incoming
    /// operation (`dst_access`). It halts the new stage until previous work finishes and ensures
    /// GPU caches are properly flushed and invalidated.
    ///
    /// # When to Use
    ///
    /// * **Stage Transitions in the Same Layout**: After a `clear_storage_texture` operation
    ///   (executing in the `TRANSFER` stage) before running a `COMPUTE_SHADER` dispatch on that texture.
    /// * **Access Mode Changes**: Transitioning from a read-only pass (e.g. `SHADER_STORAGE_READ`)
    ///   to a write pass (e.g. `SHADER_STORAGE_WRITE`) while keeping the image in `GENERAL` layout.
    /// * **Inter-Stage Reads**: Switching from a `VERTEX_SHADER` reading a texture to a `COMPUTE_SHADER`
    ///   reading the same texture in `SHADER_READ_ONLY_OPTIMAL` layout.
    ///
    /// # Contrast with [`transition_texture`](Self::transition_texture)
    ///
    /// * Use **`sync_texture`** when the texture's layout is already correct and only the stage/access mask changes.
    /// * Use **`transition_texture`** when the texture must also transition into a new [`vk::ImageLayout`].
    ///
    /// # Arguments
    ///
    /// * `texture` - The texture being synchronized. Its internal `access_state` will be updated to `dst_access`.
    /// * `dst_access` - The [`TextureAccess`] describing the stage and access mask of the upcoming pass.
    #[inline]
    pub fn sync_texture(&self, texture: &mut Texture, dst_access: TextureAccess) {
        self.insert_texture_memory_barrier(texture, None, Some(dst_access), None);
    }

    /// Release ownership of a texture from this recorder's queue to `dst_queue`.
    #[inline]
    pub fn release_texture_to_queue(&self, texture: &mut Texture, dst_queue: Arc<Queue>) {
        self.insert_texture_memory_barrier(texture, None, None, Some(dst_queue));
    }

    #[inline]
    pub fn prepare_texture_for_presentation(&self, texture: &mut Texture) {
        self.insert_texture_memory_barrier(
            texture,
            Some(vk::ImageLayout::PRESENT_SRC_KHR),
            None,
            None,
        );
    }
}

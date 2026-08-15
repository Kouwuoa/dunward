use super::command_recorder_allocator::CommandRecorderAllocator;
use crate::renderer::contexts::device_context::queue::Queue;
use crate::renderer::subsystems::resource_subsystem::resource_types::material::Material;
use crate::renderer::subsystems::resource_subsystem::resource_types::texture::{
    ColorTexture, DepthTexture, StorageTexture, Texture, TextureAccess, TextureQueueState,
};
use crate::renderer::subsystems::resource_subsystem::resource_updater::ResourceUpdater;
use ash::vk;
use color_eyre::Result;
use std::marker::PhantomData;
use std::sync::Arc;

/// Ready to record
pub(crate) struct Idle;
/// Currently recording
pub(crate) struct Recording;
/// Recording finished; can submit
pub(crate) struct Executable;

pub(crate) struct CommandRecorder<State> {
    command_buffer: vk::CommandBuffer,
    queue: Arc<Queue>,
    device: Arc<ash::Device>,
    /// Note that this is only an `Option` to allow for the allocator to be dropped.
    allocator: Option<CommandRecorderAllocator>,
    _state: PhantomData<State>,
}

impl<State> CommandRecorder<State> {
    pub fn get_queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }
    pub fn get_command_buffer(&self) -> vk::CommandBuffer {
        self.command_buffer
    }
}

impl CommandRecorder<Idle> {
    pub fn new(
        command_buffer: vk::CommandBuffer,
        queue: Arc<Queue>,
        device: Arc<ash::Device>,
        allocator: CommandRecorderAllocator,
    ) -> Self {
        Self {
            command_buffer,
            queue,
            device,
            allocator: Some(allocator),
            _state: PhantomData,
        }
    }

    pub fn new_from_old(old: CommandRecorder<Executable>) -> Self {
        Self {
            command_buffer: old.command_buffer,
            queue: old.queue,
            device: old.device,
            allocator: old.allocator,
            _state: PhantomData,
        }
    }

    pub fn record<F>(self, f: F) -> Result<CommandRecorder<Executable>>
    where
        F: FnOnce(&CommandRecorder<Recording>) -> Result<()>,
    {
        let recorder = self.begin_recording()?;

        let result = f(&recorder);

        let recorder = recorder.end_recording();

        // Propagate user error first
        result?;

        recorder
    }

    fn begin_recording(self) -> Result<CommandRecorder<Recording>> {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            self.device
                .begin_command_buffer(self.command_buffer, &begin_info)?;
        }

        Ok(CommandRecorder::<Recording> {
            command_buffer: self.command_buffer,
            queue: self.queue,
            device: self.device,
            allocator: self.allocator,
            _state: PhantomData,
        })
    }
}

impl CommandRecorder<Recording> {
    pub fn insert_texture_memory_barrier(
        &self,
        texture: &mut Texture,
        new_layout: vk::ImageLayout,
        dst_access: Option<TextureAccess>,
        dst_queue: Option<Arc<Queue>>,
    ) {
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

                if new_layout != vk::ImageLayout::PRESENT_SRC_KHR {
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
            .new_layout(new_layout)
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
        texture.layout = new_layout;
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

    pub fn blit_texture_to_texture(&self, src: &Texture, dst: &Texture) -> Result<()> {
        src.blit_to(dst, self.command_buffer);

        Ok(())
    }

    pub fn resolve_texture(
        &self,
        src: &Texture,
        src_layout: vk::ImageLayout,
        dst: &Texture,
        dst_layout: vk::ImageLayout,
        region: vk::ImageResolve,
    ) -> Result<()> {
        unsafe {
            self.device.cmd_resolve_image(
                self.command_buffer,
                src.image,
                src_layout,
                dst.image,
                dst_layout,
                &[region],
            );
        }

        Ok(())
    }

    pub fn clear_storage_texture(
        &self,
        texture: &StorageTexture,
        current_layout: vk::ImageLayout,
        color: &vk::ClearColorValue,
    ) -> Result<()> {
        unsafe {
            self.device.cmd_clear_color_image(
                self.command_buffer,
                texture.image,
                current_layout,
                color,
                &[vk::ImageSubresourceRange {
                    aspect_mask: texture.aspect,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }],
            );
        }

        Ok(())
    }

    pub fn clear_color_texture(
        &self,
        texture: &ColorTexture,
        current_layout: vk::ImageLayout,
        color: &vk::ClearColorValue,
    ) -> Result<()> {
        unsafe {
            self.device.cmd_clear_color_image(
                self.command_buffer,
                texture.image,
                current_layout,
                color,
                &[vk::ImageSubresourceRange {
                    aspect_mask: texture.aspect,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }],
            );
        }

        Ok(())
    }

    pub fn clear_depth_texture(
        &self,
        texture: &DepthTexture,
        layout: vk::ImageLayout,
    ) -> Result<()> {
        unsafe {
            self.device.cmd_clear_depth_stencil_image(
                self.command_buffer,
                texture.image,
                layout,
                &vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
                &[vk::ImageSubresourceRange {
                    aspect_mask: texture.aspect,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                }],
            );
        }

        Ok(())
    }

    /// Should be called every frame
    pub fn bind_material(&self, material: &Material) {
        material.bind_pipeline(self.command_buffer);
        material.bind_descriptor_sets(self.command_buffer);
    }

    /// Should be called every frame
    pub fn update_push_constants(&self, material: &Material, data: &[u8]) {
        material.update_push_constants(self.command_buffer, data);
    }

    pub fn dispatch(&self, group_count_x: u32, group_count_y: u32, group_count_z: u32) {
        unsafe {
            self.device.cmd_dispatch(
                self.command_buffer,
                group_count_x,
                group_count_y,
                group_count_z,
            )
        }
    }

    pub fn create_resource_updater(&self) -> ResourceUpdater<'_> {
        ResourceUpdater::new(&self.device, &self.command_buffer)
    }

    fn end_recording(self) -> Result<CommandRecorder<Executable>> {
        unsafe { self.device.end_command_buffer(self.command_buffer)? }
        Ok(CommandRecorder::<Executable> {
            command_buffer: self.command_buffer,
            queue: self.queue,
            device: self.device,
            allocator: self.allocator,
            _state: PhantomData,
        })
    }
}

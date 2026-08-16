//! Frame lighting and compute shading stage.
//!
//! Manages fullscreen render targets ([`StorageTexture`]), clears images,
//! synchronizes compute dispatches, and transfers output textures to the graphics queue.

use ash::vk;
use color_eyre::Result;

use super::packet::FrameRenderPacket;
use crate::commands::allocator::{CommandRecorderAllocator, CommandRecorderAllocatorExt};
use crate::commands::recorder::{CommandRecorder, Idle};
use crate::core::DeviceContext;
use crate::core::semaphore::TimelineSemaphore;
use crate::display::DisplayContext;
use crate::material::Material;
use crate::material::shader_data::PerDrawData;
use crate::resources::factory::ResourceFactory;
use crate::resources::store::ResourceStore;
use crate::resources::texture::{StorageTexture, TextureAccess};

pub(crate) struct FrameLightingStageOutput<'a> {
    pub(crate) target_tex: &'a mut StorageTexture,
}

pub(crate) struct FrameLightingStage {
    recorder: Option<CommandRecorder<Idle>>,

    target_tex: StorageTexture,
    target_tex_needs_update: bool,

    material: Material,
    #[allow(dead_code)]
    is_first_render: bool,
}

impl FrameLightingStage {
    pub(crate) fn new(
        dvc_ctx: &DeviceContext,
        display_ctx: &DisplayContext,
        cmd_allocator: &mut CommandRecorderAllocator,
        resource_factory: &ResourceFactory,
        resource_store: &mut ResourceStore,
    ) -> Result<Self> {
        let compute_queue = dvc_ctx.get_compute_queue();
        let recorder = Some(cmd_allocator.allocate(compute_queue)?);

        let display_size = display_ctx.get_size();
        let target_tex = resource_factory.create_storage_texture(
            display_size.width,
            display_size.height,
            true,
        )?;

        let material = resource_store
            .compute_material_factory
            .create_material()?;

        Ok(Self {
            recorder,
            target_tex,
            target_tex_needs_update: true,
            material,
            is_first_render: true,
        })
    }

    pub(crate) fn render(
        &mut self,
        pkt: FrameRenderPacket,
        dvc: &DeviceContext,
        frame_completion_timeline: &TimelineSemaphore,
        timeline_wait_val: u64,
        timeline_signal_val: u64,
    ) -> Result<FrameLightingStageOutput<'_>> {
        // Record all operations needed to run the compute shader and update the storage texture
        let recorder = self.recorder.take().unwrap();
        let recorder = recorder.record(|recorder| {
            // Check if we need to update the storage texture descriptor in the material
            if self.target_tex_needs_update {
                let mut updater = recorder.create_resource_updater();
                updater.enqueue_update(
                    |builder| {
                        builder.set_render_target_texture(&self.target_tex);
                    },
                    &self.material,
                );
                updater.execute_updates();
                self.target_tex_needs_update = false;
            }

            // Transition into GENERAL layout and prepare for the CLEAR command (TRANSFER)
            recorder.transition_texture(
                &mut self.target_tex,
                vk::ImageLayout::GENERAL,
                TextureAccess {
                    stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                },
            );

            // Clear the storage texture to black
            recorder.clear_storage_texture(&mut self.target_tex, &vk::ClearColorValue::default())?;

            // Bind the material to the command buffer
            recorder.bind_material(&self.material);

            // Update the push constants for the material
            let time_sec = pkt.time_start.elapsed().as_secs_f32();
            let per_draw_data = PerDrawData {
                object_index: 0,
                material_index: 0,
                vertex_offset: 0,
                time_sec,
            };
            recorder.update_push_constants(&self.material, per_draw_data.as_bytes());

            // Add a barrier to transition the texture from TRANSFER_WRITE to SHADER_STORAGE_WRITE
            // to ensure the clear operation has completed before the compute shader writes to it
            //
            // Texture is already in GENERAL layout from the clear operation,
            // but we need to wait on TRANSFER stage and acquire exclusive access for COMPUTE_SHADER stage
            recorder.sync_texture(
                &mut self.target_tex,
                TextureAccess {
                    stage_mask: vk::PipelineStageFlags2::COMPUTE_SHADER,
                    access_mask: vk::AccessFlags2::SHADER_STORAGE_WRITE,
                },
            );

            // Dispatch the compute shader
            let group_size_x = 16;
            let group_size_y = 16;
            let group_count_x = (pkt.target_size.width + group_size_x - 1) / group_size_x;
            let group_count_y = (pkt.target_size.height + group_size_y - 1) / group_size_y;
            recorder.dispatch(group_count_x, group_count_y, 1);

            // Release the texture from compute onto the graphics queue to match the queue of the swapchain image
            recorder.release_texture_to_queue(&mut self.target_tex, dvc.get_graphics_queue());

            Ok(())
        })?;

        // Submit the command buffer
        self.recorder = Some(dvc.submit(
            recorder,
            &[frame_completion_timeline
                .to_wait_semaphore(vk::PipelineStageFlags::COMPUTE_SHADER, timeline_wait_val)],
            &[frame_completion_timeline.to_signal_semaphore(timeline_signal_val)],
            None,
        )?);

        self.is_first_render = false;

        Ok(FrameLightingStageOutput {
            target_tex: &mut self.target_tex,
        })
    }

    pub(crate) fn resize(&mut self, size: &winit::dpi::PhysicalSize<u32>, resource_factory: &ResourceFactory) {
        self.target_tex = resource_factory
            .create_storage_texture(size.width, size.height, true)
            .unwrap();
        self.target_tex_needs_update = true;
    }
}

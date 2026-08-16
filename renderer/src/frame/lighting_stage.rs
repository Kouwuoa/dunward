//! Frame lighting and compute shading stage.
//!
//! Manages fullscreen render targets ([`StorageTexture`]), clears images,
//! synchronizes compute dispatches, and transfers output textures to the graphics queue.

use super::packet::FrameRenderPacket;
use crate::commands::CommandSubsystem;
use crate::commands::allocator::CommandRecorderAllocatorExt;
use crate::commands::recorder::{CommandRecorder, Idle};
use crate::core::DeviceContext;
use crate::core::semaphore::TimelineSemaphore;
use crate::display::DisplayContext;
use crate::material::Material;
use crate::material::shader_data::PerDrawData;
use crate::resources::ResourceSubsystem;
use crate::resources::texture::{StorageTexture, TextureAccess};
use ash::vk;
use color_eyre::Result;

pub struct FrameLightingStageOutput<'a> {
    pub target_tex: &'a mut StorageTexture,
}

pub struct FrameLightingStage {
    recorder: Option<CommandRecorder<Idle>>,

    target_tex: StorageTexture,
    target_tex_needs_update: bool,

    material: Material,
    #[allow(dead_code)]
    is_first_render: bool,
}

impl FrameLightingStage {
    pub fn new(
        dvc_ctx: &DeviceContext,
        display_ctx: &DisplayContext,
        cmd_sys: &mut CommandSubsystem,
        rsc_sys: &mut ResourceSubsystem,
    ) -> Result<Self> {
        let compute_queue = dvc_ctx.get_compute_queue();
        let recorder = Some(cmd_sys.command_recorder_allocator.allocate(compute_queue)?);

        let display_size = display_ctx.get_size();
        let target_tex = rsc_sys.resource_factory.create_storage_texture(
            display_size.width,
            display_size.height,
            true,
        )?;

        let material = rsc_sys
            .resource_store
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

    pub fn render(
        &mut self,
        pkt: FrameRenderPacket,
        dvc: &DeviceContext,
        frame_completion_timeline: &TimelineSemaphore,
        timeline_wait_val: u64,
        timeline_signal_val: u64,
    ) -> Result<FrameLightingStageOutput<'_>> {
        // Record render commands
        let recorder = self.recorder.take().unwrap();
        let recorder = recorder.record(|recorder| {
            let graphics_queue = dvc.get_graphics_queue();

            // Transition into GENERAL layout and prepare for the CLEAR command (TRANSFER)
            recorder.transition_texture(
                &mut self.target_tex,
                vk::ImageLayout::GENERAL,
                TextureAccess {
                    stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                },
            );

            // Update the render target texture if it needs updating
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

            // Clear render target texture (Runs in TRANSFER stage)
            recorder.clear_storage_texture(
                &mut self.target_tex,
                &vk::ClearColorValue {
                    float32: [1.0f32, 0.0f32, 0.0f32, 1.0f32],
                },
            )?;

            // Synchronize: Wait for CLEAR (TRANSFER) to finish before COMPUTE_SHADER runs
            // This effectively performs a flush operation to ensure the render operations that follow do not operate on stale data
            recorder.sync_texture(
                &mut self.target_tex,
                TextureAccess {
                    stage_mask: vk::PipelineStageFlags2::COMPUTE_SHADER,
                    access_mask: vk::AccessFlags2::SHADER_STORAGE_WRITE,
                },
            );

            // Compute render operations
            recorder.bind_material(&self.material);
            let per_draw_data = PerDrawData {
                time_sec: pkt.time_start.elapsed().as_secs_f32(),
                ..Default::default()
            };
            recorder.update_push_constants(&self.material, per_draw_data.as_bytes());
            let group_count_x = (self.target_tex.width() as f32 / 16.0).ceil() as u32;
            let group_count_y = (self.target_tex.height() as f32 / 16.0).ceil() as u32;
            recorder.dispatch(group_count_x, group_count_y, 1);

            // Release the texture from compute onto the graphics queue to match the queue of the swapchain image
            recorder.release_texture_to_queue(&mut self.target_tex, graphics_queue.clone());

            Ok(())
        })?;

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

    pub fn resize(&mut self, size: &winit::dpi::PhysicalSize<u32>, rsc_sys: &ResourceSubsystem) {
        self.target_tex = rsc_sys
            .resource_factory
            .create_storage_texture(size.width, size.height, true)
            .unwrap();
        self.target_tex_needs_update = true;
    }
}

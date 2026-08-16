//! Multi-buffered frame context, stage execution, synchronization, and presentation.
//!
//! Exposes [`FrameContext`], orchestrating per-frame synchronization fences, semaphores,
//! stage execution ([`FrameGeometryStage`], [`FrameLightingStage`]), and blitting to display images.

pub mod geometry_stage;
pub mod lighting_stage;
pub mod packet;

pub use geometry_stage::FrameGeometryStage;
pub use lighting_stage::FrameLightingStage;
pub use packet::{FramePresentPacket, FrameRenderPacket};

use crate::commands::CommandSubsystem;
use crate::commands::allocator::CommandRecorderAllocatorExt;
use crate::commands::recorder::{CommandRecorder, Idle};
use crate::core::DeviceContext;
use crate::core::semaphore::{BinarySemaphore, TimelineSemaphore};
use crate::display::{DisplayContext, DisplayPresentError};
use crate::resources::ResourceSubsystem;
use crate::resources::texture::TextureAccess;
use ash::vk;
use color_eyre::eyre::Result;
use std::time::Duration;

pub struct FrameContext {
    #[allow(dead_code)]
    geometry_stage: FrameGeometryStage,
    lighting_stage: FrameLightingStage,
    present_texture_acquired_semaphore: BinarySemaphore,
    previous_frame_render_finished_fence: vk::Fence,
    frame_completion_timeline: TimelineSemaphore,
    frame_completion_timeline_base: u64,
    render_finished_semaphore: BinarySemaphore,
    postrender_recorder: Option<CommandRecorder<Idle>>,
}

impl FrameContext {
    pub fn new(
        dvc_ctx: &mut DeviceContext,
        display_ctx: &DisplayContext,
        cmd_sys: &mut CommandSubsystem,
        rsc_sys: &mut ResourceSubsystem,
    ) -> Result<Self> {
        log::info!("Creating FrameContext");

        let geometry_stage = FrameGeometryStage::new(dvc_ctx, cmd_sys, rsc_sys)?;
        let lighting_stage = FrameLightingStage::new(dvc_ctx, display_ctx, cmd_sys, rsc_sys)?;

        let present_image_acquired_semaphore = dvc_ctx.create_binary_semaphore()?;
        let render_finished_semaphore = dvc_ctx.create_binary_semaphore()?;
        let previous_frame_render_finished_fence = dvc_ctx.create_vk_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
        )?;
        let frame_completion_timeline = dvc_ctx.create_timeline_semaphore()?;

        let graphics_queue = dvc_ctx.get_graphics_queue();
        let postrender_recorder = Some(
            cmd_sys
                .command_recorder_allocator
                .allocate(graphics_queue)?,
        );

        Ok(Self {
            geometry_stage,
            lighting_stage,
            present_texture_acquired_semaphore: present_image_acquired_semaphore,
            render_finished_semaphore,
            previous_frame_render_finished_fence,
            frame_completion_timeline,
            frame_completion_timeline_base: 0,
            postrender_recorder,
        })
    }

    pub fn render(
        &mut self,
        pkt: FrameRenderPacket,
        dvc: &DeviceContext,
        display: &DisplayContext,
        _rsc: &ResourceSubsystem,
    ) -> Result<FramePresentPacket> {
        let timeout = Duration::from_secs(1);

        // Wait until the commands have finished from the last time this frame was rendered
        dvc.wait_and_reset_fence(self.previous_frame_render_finished_fence, timeout)?;

        // Calculate timeline semaphore values
        let lighting_timeline_wait = self.frame_completion_timeline_base;
        let lighting_timeline_signal = self.frame_completion_timeline_base + 1;
        let postrender_timeline_signal = lighting_timeline_signal + 1;
        self.frame_completion_timeline_base = postrender_timeline_signal;

        // TODO: Render the geometry stage

        // TODO: Transfer the geometry stage output texture from graphics queue to compute queue

        // Render the lighting stage
        let lighting_stage_output = self.lighting_stage.render(
            pkt,
            dvc,
            &self.frame_completion_timeline,
            lighting_timeline_wait,
            lighting_timeline_signal,
        )?;

        // Acquire the next image from the display swapchain
        let mut present_tex =
            display.acquire_next_present_texture(&self.present_texture_acquired_semaphore, timeout)?;

        // Perform post-render operations on the compute queue
        let postrender_recorder = self.postrender_recorder.take().unwrap();
        let postrender_recorder = postrender_recorder.record(|recorder| {
            // Transition the image layout into TRANSFER_DST_OPTIMAL for present texture to prepare for blit
            recorder.transition_texture(
                &mut present_tex.texture,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                TextureAccess {
                    stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    access_mask: vk::AccessFlags2::TRANSFER_WRITE,
                },
            );

            // Acquire the lighting output texture from the compute queue
            // Also transition render target texture to transfer source layout to prepare for blitting onto display image
            recorder.transition_texture(
                lighting_stage_output.target_tex,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                TextureAccess {
                    stage_mask: vk::PipelineStageFlags2::TRANSFER,
                    access_mask: vk::AccessFlags2::TRANSFER_READ,
                },
            );

            recorder.blit_texture_to_texture(
                lighting_stage_output.target_tex,
                &mut present_tex.texture,
            )?;

            // Prepare display texture for presentation
            recorder.prepare_texture_for_presentation(&mut present_tex.texture);

            // Release the lighting output texture back to the compute queue
            recorder.release_texture_to_queue(
                lighting_stage_output.target_tex,
                dvc.get_compute_queue(),
            );

            Ok(())
        })?;

        self.postrender_recorder = Some(dvc.submit(
            postrender_recorder,
            &[
                // Wait for the lighting stage to finish rendering
                self.frame_completion_timeline.to_wait_semaphore(
                    vk::PipelineStageFlags::TRANSFER,
                    lighting_timeline_signal,
                ),
                // Wait for the present texture to be acquired
                self.present_texture_acquired_semaphore
                    .to_wait_semaphore(vk::PipelineStageFlags::TRANSFER),
            ],
            // Signal that all render operations have finished, meaning the display image is ready to be presented
            &[
                self.frame_completion_timeline
                    .to_signal_semaphore(postrender_timeline_signal),
                self.render_finished_semaphore.to_signal_semaphore(),
            ],
            Some(self.previous_frame_render_finished_fence),
        )?);

        Ok(FramePresentPacket {
            texture: present_tex,
        })
    }

    pub fn present(
        &self,
        pkt: FramePresentPacket,
        display: &DisplayContext,
    ) -> core::result::Result<(), DisplayPresentError> {
        display.present(pkt.texture, &self.render_finished_semaphore)
    }

    pub fn resize(&mut self, size: &winit::dpi::PhysicalSize<u32>, rsc_sys: &ResourceSubsystem) {
        // TODO: Resize the geometry stage as well
        self.lighting_stage.resize(size, rsc_sys);
    }

    pub fn destroy(mut self, cmd_sys: &mut CommandSubsystem) -> Result<()> {
        if let Some(postrender_recorder) = self.postrender_recorder.take() {
            cmd_sys
                .command_recorder_allocator
                .deallocate(postrender_recorder)?;
        }
        Ok(())
    }
}

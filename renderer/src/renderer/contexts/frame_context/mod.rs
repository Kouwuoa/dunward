pub(crate) mod packet;

mod frame_geometry_stage;
mod frame_lighting_stage;

use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::contexts::device_context::semaphore::{BinarySemaphore, TimelineSemaphore};
use crate::renderer::contexts::frame_context::frame_geometry_stage::FrameGeometryStage;
use crate::renderer::contexts::frame_context::frame_lighting_stage::FrameLightingStage;
use crate::renderer::contexts::frame_context::packet::{FramePresentPacket, FrameRenderPacket};
use crate::renderer::contexts::swapchain_context::{SwapchainContext, SwapchainPresentError};
use crate::renderer::subsystems::command_subsystem::CommandSubsystem;
use crate::renderer::subsystems::command_subsystem::command_recorder::{CommandRecorder, Idle};
use crate::renderer::subsystems::command_subsystem::command_recorder_allocator::CommandRecorderAllocatorExt;
use crate::renderer::subsystems::resource_subsystem::ResourceSubsystem;
use ash::vk;
use color_eyre::eyre::Result;
use std::time::Duration;

pub(crate) struct FrameContext {
    geometry_stage: FrameGeometryStage,
    lighting_stage: FrameLightingStage,
    present_image_acquired_semaphore: BinarySemaphore,
    render_finished_semaphore: BinarySemaphore, // Used for
    previous_frame_render_finished_fence: vk::Fence,
    frame_completion_timeline: TimelineSemaphore,
    frame_completion_timeline_base: u64,
    postrender_recorder: Option<CommandRecorder<Idle>>,
}

impl FrameContext {
    const RENDER_STAGE_COUNT: u64 = 2;

    pub fn new(
        dvc_ctx: &mut DeviceContext,
        swc_ctx: &SwapchainContext,
        cmd_sys: &mut CommandSubsystem,
        rsc_sys: &mut ResourceSubsystem,
    ) -> Result<Self> {
        log::info!("Creating FrameContext");

        let geometry_stage = FrameGeometryStage::new(dvc_ctx, cmd_sys, rsc_sys)?;
        let lighting_stage = FrameLightingStage::new(dvc_ctx, swc_ctx, cmd_sys, rsc_sys)?;

        let present_image_acquired_semaphore = dvc_ctx.create_binary_semaphore()?;
        let render_finished_semaphore = dvc_ctx.create_binary_semaphore()?;
        let previous_frame_render_finished_fence = dvc_ctx.create_vk_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
        )?;
        let frame_completion_timeline = dvc_ctx.create_timeline_semaphore()?;

        let compute_queue = dvc_ctx.get_compute_queue();
        let postrender_recorder = Some(cmd_sys.command_recorder_allocator.allocate(compute_queue)?);

        Ok(Self {
            geometry_stage,
            lighting_stage,
            present_image_acquired_semaphore,
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
        swc: &SwapchainContext,
        rsc: &ResourceSubsystem,
    ) -> Result<FramePresentPacket> {
        let timeout = Duration::from_secs(1);

        // Wait until the commands have finished from the last time this frame was rendered
        dvc.wait_and_reset_fence(self.previous_frame_render_finished_fence, timeout)?;

        // Calculate timeline semaphore values
        let lighting_timeline_wait = self.frame_completion_timeline_base;
        let lighting_timeline_signal =
            self.frame_completion_timeline_base + FrameLightingStage::TIMELINE_SEM_SIGNAL_OFFSET;
        self.frame_completion_timeline_base = lighting_timeline_signal;

        // TODO: Render the geometry stage

        // TODO: Transfer the geometry stage output texture from graphics queue to compute queue

        // Render the lighting stage
        let mut lighting_stage_output = self.lighting_stage.render(
            pkt,
            dvc,
            &self.frame_completion_timeline,
            lighting_timeline_wait,
            lighting_timeline_signal,
        )?;

        // Acquire the next image from the swapchain
        let mut present_tex =
            swc.acquire_next_present_texture(&self.present_image_acquired_semaphore, timeout)?;

        // Perform post-render operations on the compute queue
        let postrender_recorder = self.postrender_recorder.take().unwrap();
        let postrender_recorder = postrender_recorder.record(|recorder| {
            // Transition the image layout for present texture
            recorder.transition_texture_layout(
                &mut present_tex.texture,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            )?;

            // TODO: Transfer ownership of lighting stage output texture from compute to graphics queue family
            // We need to do this because the texture copy operation requires both textures to be in the same queue family,
            // and the present texture (i.e. swapchain image) is in the graphics queue

            recorder.copy_texture_to_texture(
                lighting_stage_output.target_tex,
                &mut present_tex.texture,
            )?;

            // Prepare swapchain texture for presentation
            recorder.transition_texture_layout(
                &mut present_tex.texture,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            )?;
            Ok(())
        })?;

        self.postrender_recorder = Some(
            dvc.submit(
                postrender_recorder,
                &[self
                    .present_image_acquired_semaphore
                    .to_wait_semaphore(vk::PipelineStageFlags::TRANSFER)],
                &[self.render_finished_semaphore.to_signal_semaphore()],
                self.previous_frame_render_finished_fence,
            )?,
        );

        Ok(FramePresentPacket {
            texture: present_tex,
        })
    }

    pub fn present(
        &self,
        pkt: FramePresentPacket,
        swc: &SwapchainContext,
    ) -> core::result::Result<(), SwapchainPresentError> {
        swc.present(pkt.texture, &self.render_finished_semaphore)
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

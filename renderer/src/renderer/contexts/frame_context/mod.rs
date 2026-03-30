pub(crate) mod packet;

mod frame_geometry_stage;
mod frame_lighting_stage;

use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::contexts::frame_context::frame_geometry_stage::FrameGeometryStage;
use crate::renderer::contexts::frame_context::frame_lighting_stage::FrameLightingStage;
use crate::renderer::contexts::frame_context::packet::{FramePresentPacket, FrameRenderPacket};
use crate::renderer::contexts::swapchain_context::{SwapchainContext, SwapchainPresentError};
use crate::renderer::subsystems::command_subsystem::CommandSubsystem;
use crate::renderer::subsystems::command_subsystem::command_recorder_allocator::CommandRecorderAllocatorExt;
use crate::renderer::subsystems::resource_subsystem::ResourceSubsystem;
use crate::renderer::subsystems::resource_subsystem::resource_types::shader_data::PerDrawData;
use ash::vk;
use color_eyre::eyre::Result;
use std::time::Duration;

pub(crate) struct FrameContext {
    geometry_stage: FrameGeometryStage,
    lighting_stage: FrameLightingStage,
    present_image_acquired_semaphore: vk::Semaphore,
    previous_frame_render_finished_fence: vk::Fence,
    timeline_semaphore: vk::Semaphore,
}

impl FrameContext {
    pub fn new(
        dvc_ctx: &mut DeviceContext,
        swc_ctx: &SwapchainContext,
        cmd_sys: &mut CommandSubsystem,
        rsc_sys: &mut ResourceSubsystem,
    ) -> Result<Self> {
        log::info!("Creating FrameContext");

        let geometry_stage = FrameGeometryStage::new(dvc_ctx, cmd_sys, rsc_sys)?;
        let lighting_stage = FrameLightingStage::new(dvc_ctx, swc_ctx, cmd_sys, rsc_sys)?;

        let present_image_acquired_semaphore =
            dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let previous_frame_render_finished_fence = dvc_ctx.create_vk_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
        )?;
        let timeline_semaphore = dvc_ctx.create_vk_semaphore(
            &vk::SemaphoreCreateInfo::default().push_next(&mut vk::SemaphoreTypeCreateInfo {
                semaphore_type: vk::SemaphoreType::TIMELINE,
                initial_value: 0,
                ..Default::default()
            }),
        )?;

        Ok(Self {
            geometry_stage,
            lighting_stage,
            present_image_acquired_semaphore,
            previous_frame_render_finished_fence,
            timeline_semaphore,
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

        {
            // TODO: Perform render operations here

            // Acquire the next image from the swapchain
            let mut present_tex = swc.acquire_next_present_texture(
                self.graphics.present_image_acquired_semaphore,
                timeout,
            )?;

            // Copy draw_color_tex onto swapchain texture
            recorder.transition_texture_layout(
                &mut present_tex.texture,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            )?;
            recorder.copy_texture_to_texture(
                &self.compute.render_target_tex,
                &mut present_tex.texture,
            )?;

            // Prepare swapchain texture for presentation
            recorder.transition_texture_layout(
                &mut present_tex.texture,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            )?;
        }

        self.lighting_stage.recorder = Some(dvc.submit(
            recorder,
            &[self.graphics.present_image_acquired_semaphore],
            &[self.graphics.graphics_finished_semaphore],
            self.graphics.graphics_finished_fence,
        )?);

        Ok(FramePresentPacket {
            texture: present_tex,
        })
    }

    pub fn present(
        &self,
        pkt: FramePresentPacket,
        swc: &SwapchainContext,
    ) -> core::result::Result<(), SwapchainPresentError> {
        swc.present(pkt.texture, self.graphics.graphics_finished_semaphore)
    }

    pub fn resize(&mut self, size: &winit::dpi::PhysicalSize<u32>, rsc_sys: &ResourceSubsystem) {
        self.compute.render_target_tex = rsc_sys
            .resource_factory
            .create_storage_texture(size.width, size.height, true)
            .unwrap();
        self.compute.render_target_tex_needs_update = true;
    }

    pub fn destroy(mut self, cmd_sys: &mut CommandSubsystem) -> Result<()> {
        if let Some(graphics_recorder) = self.graphics.recorder.take() {
            cmd_sys
                .command_recorder_allocator
                .deallocate(graphics_recorder)?;
        }
        if let Some(compute_recorder) = self.compute.recorder.take() {
            cmd_sys
                .command_recorder_allocator
                .deallocate(compute_recorder)?;
        }
        Ok(())
    }
}

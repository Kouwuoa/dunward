pub(crate) mod packet;
mod frame_geometry_stage;
mod frame_lighting_stage;

use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::contexts::frame_context::packet::{FramePresentPacket, FrameRenderPacket};
use crate::renderer::contexts::swapchain_context::{SwapchainContext, SwapchainPresentError};
use crate::renderer::subsystems::command_subsystem::CommandSubsystem;
use crate::renderer::subsystems::command_subsystem::command_recorder::{CommandRecorder, Idle};
use crate::renderer::subsystems::command_subsystem::command_recorder_allocator::CommandRecorderAllocatorExt;
use crate::renderer::subsystems::resource_subsystem::ResourceSubsystem;
use crate::renderer::subsystems::resource_subsystem::resource_types::material::Material;
use crate::renderer::subsystems::resource_subsystem::resource_types::megabuffer::{
    AllocatedMegabufferRegion, MegabufferExt,
};
use crate::renderer::subsystems::resource_subsystem::resource_types::shader_data::PerDrawData;
use crate::renderer::subsystems::resource_subsystem::resource_types::texture::StorageTexture;
use ash::vk;
use color_eyre::eyre::Result;
use std::time::Duration;
use crate::renderer::contexts::frame_context::frame_geometry_stage::FrameGeometryStage;
use crate::renderer::contexts::frame_context::frame_lighting_stage::FrameLightingStage;

pub(crate) struct FrameContext {
    geometry_stage: FrameGeometryStage,
    lighting_stage: FrameLightingStage,
    present_image_acquired_semaphore: vk::Semaphore,
    previous_frame_render_finished_fence: vk::Fence,
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
        let present_image_acquired_semaphore = dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;

        Ok(Self {
            geometry_stage,
            lighting_stage,
            present_image_acquired_semaphore,
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
        dvc.wait_and_reset_fence(self.graphics.graphics_finished_fence, timeout)?;

        // Acquire the next image from the swapchain
        let mut present_tex = swc.acquire_next_present_texture(
            self.graphics.present_image_acquired_semaphore,
            timeout,
        )?;

        // Record render commands
        let recorder = self.graphics.recorder.take().unwrap();
        let recorder = recorder.record(|recorder| {
            // Transition render target texture to GENERAL layout
            recorder.transition_texture_layout(
                &mut self.compute.render_target_tex,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            )?;

            // Update the render target texture if it needs updating
            if self.compute.render_target_tex_needs_update {
                let mut updater = recorder.create_resource_updater();
                updater.enqueue_update(
                    |builder| {
                        builder.set_render_target_texture(&self.compute.render_target_tex);
                    },
                    &self.compute.bindless_material,
                );
                updater.execute_updates();
                self.compute.render_target_tex_needs_update = false;
            }

            // Clear render target texture
            recorder.clear_storage_texture(
                &self.compute.render_target_tex,
                vk::ImageLayout::GENERAL,
                &vk::ClearColorValue {
                    float32: [1.0f32, 0.0f32, 0.0f32, 1.0f32],
                },
            )?;

            // Insert memory barrier that waits until the storage texture has been fully cleared before continuing with read/write operations
            recorder.insert_texture_memory_barrier(
                &self.compute.render_target_tex,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::GENERAL,
                vk::PipelineStageFlags2::TRANSFER,
                vk::AccessFlags2::TRANSFER_WRITE,
                vk::PipelineStageFlags2::COMPUTE_SHADER,
                vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::SHADER_STORAGE_WRITE,
            );

            // Compute render operations
            recorder.bind_material(&self.compute.bindless_material);
            let per_draw_data = PerDrawData {
                time_sec: pkt.time_start.elapsed().as_secs_f32(),
                ..Default::default()
            };
            recorder.update_push_constants(&self.compute.bindless_material, per_draw_data.as_bytes());
            let group_count_x =
                (self.compute.render_target_tex.width() as f32 / 16.0).ceil() as u32;
            let group_count_y =
                (self.compute.render_target_tex.height() as f32 / 16.0).ceil() as u32;
            recorder.dispatch(group_count_x, group_count_y, 1);

            // TODO: Perform render operations here

            // Copy draw_color_tex onto swapchain texture
            recorder.transition_texture_layout(
                &mut self.compute.render_target_tex,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            )?;
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

            Ok(())
        })?;

        self.graphics.recorder = Some(dvc.submit(
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

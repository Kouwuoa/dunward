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

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_FRAME_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_MATERIAL_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_OBJECT_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB

pub(crate) struct FrameContext {
    geometry_stage: FrameGeometryStage,
    lighting_stage: FrameLightingStage,
    present_image_acquired_semaphore: vk::Semaphore,
}

impl FrameContext {
    pub fn new(
        dvc_ctx: &mut DeviceContext,
        swc_ctx: &SwapchainContext,
        cmd_sys: &mut CommandSubsystem,
        rsc_sys: &mut ResourceSubsystem,
    ) -> Result<Self> {
        log::info!("Creating FrameContext");


        let vertex_region = rsc_sys
            .resource_store
            .vertex_megabuffer
            .allocate_region(FRAME_VERTEX_BUFFER_SIZE)?;
        let index_region = rsc_sys
            .resource_store
            .index_megabuffer
            .allocate_region(FRAME_INDEX_BUFFER_SIZE)?;
        let per_frame_region = rsc_sys
            .resource_store
            .per_frame_megabuffer
            .allocate_region(FRAME_PER_FRAME_BUFFER_SIZE)?;
        let per_material_region = rsc_sys
            .resource_store
            .per_material_megabuffer
            .allocate_region(FRAME_PER_MATERIAL_BUFFER_SIZE)?;
        let per_object_region = rsc_sys
            .resource_store
            .per_object_megabuffer
            .allocate_region(FRAME_PER_OBJECT_BUFFER_SIZE)?;

        let present_image_acquired_semaphore = dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let graphics_finished_semaphore = dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let graphics_finished_fence = dvc_ctx.create_vk_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
        )?;
        let compute_finished_semaphore = dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let compute_finished_fence = dvc_ctx.create_vk_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
        )?;

        // Note: Even though we're using compute shaders-temp,
        // we'll still use the graphics queue to avoid queue family ownership transfers and semaphore complexity.
        // The graphics queue should support compute operations.
        let graphics_queue = dvc_ctx.get_graphics_queue();
        assert!(graphics_queue.family.supports_compute());
        let graphics_recorder = Some(
            cmd_sys
                .command_recorder_allocator
                .allocate(graphics_queue)?,
        );
        let compute_recorder = Some(
            cmd_sys
                .command_recorder_allocator
                .allocate(dvc_ctx.get_graphics_queue())?,
        );

        Ok(Self {
            graphics: FrameGraphicsPart {
                recorder: graphics_recorder,
                vertex_region,
                index_region,
                per_frame_region,
                per_material_region,
                per_object_region,
                present_image_acquired_semaphore,
                graphics_finished_semaphore,
                graphics_finished_fence,
            },
            compute: FrameComputePart {
                recorder: compute_recorder,
                render_target_tex,
                render_target_tex_needs_update: true,
                compute_finished_semaphore,
                compute_finished_fence,
                bindless_material,
            },
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

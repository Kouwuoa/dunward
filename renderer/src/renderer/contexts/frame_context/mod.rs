pub(crate) mod packet;

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
use crate::renderer::subsystems::resource_subsystem::resource_updater::ResourceUpdater;
use ash::vk;
use color_eyre::eyre::Result;
use std::time::Duration;

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_FRAME_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_MATERIAL_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_OBJECT_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB

pub(crate) struct FrameContext {
    graphics_recorder: Option<CommandRecorder<Idle>>,
    render_target_tex: StorageTexture,
    render_target_tex_needs_update: bool,

    vertex_region: AllocatedMegabufferRegion,
    index_region: AllocatedMegabufferRegion,
    per_frame_region: AllocatedMegabufferRegion,
    per_material_region: AllocatedMegabufferRegion,
    per_object_region: AllocatedMegabufferRegion,

    /// Signals when the swapchain is ready to present (i.e. when the next swapchain image has been acquired successfully).
    present_semaphore: vk::Semaphore,
    /// Signals when rendering commands have been submitted to a queue.
    render_semaphore: vk::Semaphore,
    /// Signals when all rendering commands have finished execution.
    render_fence: vk::Fence,

    bindless_material: Material,
}

impl FrameContext {
    pub fn new(
        dvc_ctx: &mut DeviceContext,
        swc_ctx: &SwapchainContext,
        cmd_sys: &mut CommandSubsystem,
        rsc_sys: &mut ResourceSubsystem,
    ) -> Result<Self> {
        log::info!("Creating FrameContext");

        let swc_size = swc_ctx.get_size();
        let render_target_tex = rsc_sys.resource_factory.create_storage_texture(
            swc_size.width,
            swc_size.height,
            true,
        )?;

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

        let present_semaphore = dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let render_semaphore = dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let render_fence = dvc_ctx.create_vk_fence(
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

        let bindless_material = rsc_sys
            .resource_store
            .bindless_material_factory
            .create_material()?;

        Ok(Self {
            graphics_recorder,
            render_target_tex,
            render_target_tex_needs_update: true,

            vertex_region,
            index_region,
            per_frame_region,
            per_material_region,
            per_object_region,

            present_semaphore,
            render_semaphore,
            render_fence,

            bindless_material,
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
        dvc.wait_and_reset_fence(self.render_fence, timeout)?;

        // Acquire the next image from the swapchain
        let mut present_tex = swc.acquire_next_present_texture(self.present_semaphore, timeout)?;

        // Record render commands
        let recorder = self.graphics_recorder.take().unwrap();
        let recorder = recorder.record(|recorder| {
            // Transition render target texture to GENERAL layout
            recorder.transition_texture_layout(
                &mut self.render_target_tex,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            )?;

            // Update the render target texture if it needs updating
            if self.render_target_tex_needs_update {
                let mut updater = recorder.create_resource_updater();
                updater.enqueue_update(
                    |builder| {
                        builder.set_render_target_texture(&self.render_target_tex);
                    },
                    &self.bindless_material,
                );
                updater.execute_updates();
                self.render_target_tex_needs_update = false;
            }

            // Clear render target texture
            recorder.clear_storage_texture(
                &self.render_target_tex,
                vk::ImageLayout::GENERAL,
                &vk::ClearColorValue {
                    float32: [1.0f32, 0.0f32, 0.0f32, 1.0f32],
                },
            )?;

            // Insert memory barrier that waits until the storage texture has been fully cleared before continuing with read/write operations
            recorder.insert_texture_memory_barrier(
                &self.render_target_tex,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::GENERAL,
                vk::PipelineStageFlags2::TRANSFER,
                vk::AccessFlags2::TRANSFER_WRITE,
                vk::PipelineStageFlags2::COMPUTE_SHADER,
                vk::AccessFlags2::SHADER_STORAGE_READ | vk::AccessFlags2::SHADER_STORAGE_WRITE,
            );

            // Compute render operations
            recorder.bind_material(&self.bindless_material);
            let per_draw_data = PerDrawData {
                time_sec: pkt.time_start.elapsed().as_secs_f32(),
                ..Default::default()
            };
            recorder.update_push_constants(&self.bindless_material, per_draw_data.as_bytes());
            let group_count_x = (self.render_target_tex.width() as f32 / 16.0).ceil() as u32;
            let group_count_y = (self.render_target_tex.height() as f32 / 16.0).ceil() as u32;
            recorder.dispatch(group_count_x, group_count_y, 1);

            // TODO: Perform render operations here

            // Copy draw_color_tex onto swapchain texture
            recorder.transition_texture_layout(
                &mut self.render_target_tex,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            )?;
            recorder.transition_texture_layout(
                &mut present_tex.texture,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            )?;
            recorder.copy_texture_to_texture(&self.render_target_tex, &mut present_tex.texture)?;

            // Prepare swapchain texture for presentation
            recorder.transition_texture_layout(
                &mut present_tex.texture,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            )?;

            Ok(())
        })?;

        self.graphics_recorder = Some(dvc.submit(
            recorder,
            &[self.present_semaphore],
            &[self.render_semaphore],
            self.render_fence,
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
        swc.present(pkt.texture, self.render_semaphore)
    }

    pub fn resize(&mut self, size: &winit::dpi::PhysicalSize<u32>, rsc_sys: &ResourceSubsystem) {
        self.render_target_tex = rsc_sys
            .resource_factory
            .create_storage_texture(size.width, size.height, true)
            .unwrap();
        self.render_target_tex_needs_update = true;
    }

    pub fn destroy(mut self, cmd_sys: &mut CommandSubsystem) -> Result<()> {
        if let Some(graphics_recorder) = self.graphics_recorder.take() {
            cmd_sys
                .command_recorder_allocator
                .deallocate(graphics_recorder)?;
        }
        Ok(())
    }
}

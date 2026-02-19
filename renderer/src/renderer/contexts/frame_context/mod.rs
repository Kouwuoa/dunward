pub(crate) mod packet;

use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::contexts::device_context::commands::{
    CommandRecorder, CommandRecorderAllocatorExt, Idle,
};
use crate::renderer::contexts::frame_context::packet::{FramePresentPacket, FrameRenderPacket};
use crate::renderer::contexts::swapchain_context::{SwapchainContext, SwapchainPresentError};
use crate::renderer::resource_store::ResourceStore;
use crate::renderer::resource_store::material::Material;
use crate::renderer::resource_store::megabuffer::{AllocatedMegabufferRegion, MegabufferExt};
use crate::renderer::resource_store::shader_data::PerDrawData;
use crate::renderer::resource_store::texture::{ColorTexture, DepthTexture, StorageTexture};
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
    draw_tex: StorageTexture,

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
        rsc_sto: &mut ResourceStore,
    ) -> Result<Self> {
        log::info!("Creating FrameContext");

        let swc_size = swc_ctx.get_size();
        let draw_tex = dvc_ctx.create_storage_texture(swc_size.width, swc_size.height, true)?;

        let vertex_region = rsc_sto
            .vertex_megabuffer
            .allocate_region(FRAME_VERTEX_BUFFER_SIZE)?;
        let index_region = rsc_sto
            .index_megabuffer
            .allocate_region(FRAME_INDEX_BUFFER_SIZE)?;
        let per_frame_region = rsc_sto
            .per_frame_megabuffer
            .allocate_region(FRAME_PER_FRAME_BUFFER_SIZE)?;
        let per_material_region = rsc_sto
            .per_material_megabuffer
            .allocate_region(FRAME_PER_MATERIAL_BUFFER_SIZE)?;
        let per_object_region = rsc_sto
            .per_object_megabuffer
            .allocate_region(FRAME_PER_OBJECT_BUFFER_SIZE)?;

        let present_semaphore = dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let render_semaphore = dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let render_fence = dvc_ctx.create_vk_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
        )?;

        // Note: Even though we're using compute shaders,
        // we'll still use the graphics queue to avoid queue family ownership transfers and semaphore complexity.
        // The graphics queue should support compute operations.
        let graphics_queue = dvc_ctx.get_graphics_queue();
        assert!(graphics_queue.family.supports_compute());
        let graphics_recorder = Some(
            dvc_ctx
                .command_recorder_allocator
                .allocate(graphics_queue)?,
        );

        let bindless_material = rsc_sto.bindless_material_factory.create_material()?;

        Ok(Self {
            graphics_recorder,
            draw_tex,

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
        rsc: &ResourceStore,
    ) -> Result<FramePresentPacket> {
        let timeout = Duration::from_secs(1);

        // Wait until the commands have finished from the last time this frame was rendered
        dvc.wait_and_reset_fence(self.render_fence, timeout)?;

        // Acquire the next image from the swapchain
        let mut present_tex =
            swc.acquire_next_present_texture(self.present_semaphore, timeout, dvc)?;

        // Record render commands
        let recorder = self.graphics_recorder.take().unwrap();
        let recorder = recorder.record(|recorder| {
            recorder.transition_texture_layout(
                &mut self.draw_tex,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::GENERAL,
            )?;
            recorder.clear_storage_texture(
                &self.draw_tex,
                vk::ImageLayout::GENERAL,
                &vk::ClearColorValue {
                    float32: [1.0f32, 0.0f32, 0.0f32, 1.0f32],
                },
            )?;

            // Compute render operations
            recorder.bind_material(&self.bindless_material);
            recorder
                .update_push_constants(&self.bindless_material, PerDrawData::default().as_bytes());
            recorder.dispatch(16, 16, 0);

            /*
            recorder.transition_texture_layout(
                &mut self.draw_tex,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            )?;
             */

            // TODO: Perform render operations here

            // Copy draw_color_tex onto swapchain texture
            recorder.transition_texture_layout(
                &mut self.draw_tex,
                vk::ImageLayout::GENERAL,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
            )?;
            recorder.transition_texture_layout(
                &mut present_tex.texture,
                vk::ImageLayout::UNDEFINED,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            )?;
            recorder.copy_texture_to_texture(&self.draw_tex, &mut present_tex.texture)?;

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

    pub fn destroy(mut self, dvc_ctx: &mut DeviceContext) -> Result<()> {
        if let Some(graphics_recorder) = self.graphics_recorder.take() {
            dvc_ctx
                .command_recorder_allocator
                .deallocate(&graphics_recorder)?;
        }
        Ok(())
    }
}

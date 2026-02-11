pub(crate) mod packet;

use crate::contexts::device_context::commands::CommandRecorderAllocatorExt;
use crate::contexts::swapchain_context::SwapchainPresentError;
use crate::{
    contexts::{
        device_context::{DeviceContext, commands::CommandRecorder},
        frame_context::packet::{FramePresentPacket, FrameRenderPacket},
        swapchain_context::SwapchainContext,
    },
    resource_store::{
        ResourceStore,
        material::Material,
        megabuffer::MegabufferExt,
        megabuffer::{AllocatedMegabufferRegion, Megabuffer},
        texture::{ColorTexture, DepthTexture, Texture},
    },
    utils::GuardResultExt,
};
use ash::vk;
use color_eyre::eyre::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_FRAME_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_MATERIAL_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_OBJECT_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB

pub(crate) struct FrameContext {
    dvc_ctx: Arc<Mutex<DeviceContext>>,
    swc_ctx: Arc<Mutex<SwapchainContext>>,
    rsc_sto: Arc<Mutex<ResourceStore>>,

    command_recorder: CommandRecorder,

    draw_color_tex: ColorTexture,
    draw_depth_tex: DepthTexture,

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
        dvc_ctx: Arc<Mutex<DeviceContext>>,
        swc_ctx: Arc<Mutex<SwapchainContext>>,
        rsc_sto: Arc<Mutex<ResourceStore>>,
    ) -> Result<Self> {
        log::info!("Creating FrameContext");

        let mut dvc_grd = dvc_ctx.lock().eyre()?;
        let mut swc_grd = swc_ctx.lock().eyre()?;
        let mut rsc_grd = rsc_sto.lock().eyre()?;

        let swc_size = swc_grd.get_size();
        let draw_color_tex =
            dvc_grd.create_color_texture(swc_size.width, swc_size.height, None, true)?;
        let draw_depth_tex = dvc_grd.create_depth_texture(swc_size.width, swc_size.height)?;

        let vertex_region = rsc_grd
            .vertex_megabuffer
            .allocate_region(FRAME_VERTEX_BUFFER_SIZE)?;
        let index_region = rsc_grd
            .index_megabuffer
            .allocate_region(FRAME_INDEX_BUFFER_SIZE)?;
        let per_frame_region = rsc_grd
            .per_frame_megabuffer
            .allocate_region(FRAME_PER_FRAME_BUFFER_SIZE)?;
        let per_material_region = rsc_grd
            .per_material_megabuffer
            .allocate_region(FRAME_PER_MATERIAL_BUFFER_SIZE)?;
        let per_object_region = rsc_grd
            .per_object_megabuffer
            .allocate_region(FRAME_PER_OBJECT_BUFFER_SIZE)?;

        let present_semaphore = dvc_grd.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let render_semaphore = dvc_grd.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let render_fence = dvc_grd.create_vk_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
        )?;

        let graphics_queue = dvc_grd.get_graphics_queue();
        let command_recorder = dvc_grd
            .command_recorder_allocator
            .allocate(graphics_queue)?;

        let bindless_material = rsc_grd.bindless_material_factory.create_material()?;

        drop(dvc_grd);
        drop(swc_grd);
        drop(rsc_grd);

        Ok(Self {
            dvc_ctx,
            rsc_sto,
            swc_ctx,

            command_recorder,

            draw_color_tex,
            draw_depth_tex,

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

    pub fn render(&mut self, pkt: FrameRenderPacket) -> Result<FramePresentPacket> {
        let dvc = self.dvc_ctx.lock().eyre()?;
        let swc = self.swc_ctx.lock().eyre()?;

        let timeout = Duration::from_secs(1);

        // Wait until the commands have finished from the last time this frame was rendered
        dvc.wait_and_reset_fence(self.render_fence, timeout)?;

        // Acquire the next image from the swapchain
        let mut texture = swc
            .acquire_next_present_texture(self.present_semaphore, timeout, &dvc)?;

        // Record render commands
        self.command_recorder.begin_recording()?;

        self.command_recorder.transition_texture_layout(
            &mut texture.texture,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );

        // TODO: Perform render operations here

        self.command_recorder.transition_texture_layout(
            &mut texture.texture,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );

        let cmd = self.command_recorder.end_recording()?;

        // Submit render commands
        dvc.submit(
            cmd,
            dvc.get_graphics_queue(),
            &[self.present_semaphore],
            &[self.render_semaphore],
            self.render_fence,
        )?;

        Ok(FramePresentPacket { texture })
    }

    pub fn present(
        &self,
        pkt: FramePresentPacket,
    ) -> core::result::Result<(), SwapchainPresentError> {
        let swc = self.swc_ctx.lock().unwrap();
        swc.present(pkt.texture, self.render_semaphore)
    }
}

pub(crate) mod packet;

use crate::contexts::device_context::DeviceContext;
use crate::contexts::device_context::commands::CommandEncoder;
use crate::contexts::frame_context::packet::{FramePresentPacket, FrameRenderPacket};
use crate::contexts::swapchain_context::{PresentResult, SwapchainContext};
use crate::resource_store::ResourceStore;
use crate::resources::material::Material;
use crate::resources::megabuffer::MegabufferExt;
use crate::resources::megabuffer::{AllocatedMegabufferRegion, Megabuffer};
use crate::resources::texture::{ColorTexture, DepthTexture, Texture};
use crate::utils::GuardResultExt;
use ash::vk;
use color_eyre::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_FRAME_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_MATERIAL_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_OBJECT_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB

pub(crate) struct FrameContext {
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

    cmd_encoder: CommandEncoder,
    bindless_material: Material,

    ctx: Arc<Mutex<DeviceContext>>,
    vpt: Arc<Mutex<SwapchainContext>>,
    sto: Arc<Mutex<ResourceStore>>,
}

impl FrameContext {
    pub fn new(
        dvc_ctx: Arc<Mutex<DeviceContext>>,
        swc_ctx: Arc<Mutex<SwapchainContext>>,
        rsc_sto: Arc<Mutex<ResourceStore>>,
    ) -> Result<Self> {
        log::info!("Creating RenderFrame");

        let mut dvc_grd = dvc_ctx.lock().eyre()?;
        let mut swc_grd = swc_ctx.lock().eyre()?;
        let mut rsc_grd = rsc_sto.lock().eyre()?;

        let vpt_size = swc_grd.get_size();
        let draw_color_tex =
            dvc_grd
                .create_color_texture(vpt_size.width, vpt_size.height, None, true)?;
        let draw_depth_tex = dvc_grd
            .dev
            .create_depth_texture(vpt_size.width, vpt_size.height)?;

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

        let present_semaphore = unsafe {
            dvc_grd
                .dev
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };
        let render_semaphore = unsafe {
            dvc_grd
                .dev
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };
        let render_fence = unsafe {
            dvc_grd.dev.logical.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };

        let graphics_queue = dvc_grd.dev.graphics_queue.clone();
        let cmd_encoder = dvc_grd.dev.allocate_command_encoder(graphics_queue)?;

        let bindless_material = rsc_grd.bindless_material_factory.create_material()?;

        drop(dvc_grd);
        drop(swc_grd);
        drop(rsc_grd);

        Ok(Self {
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

            cmd_encoder,
            bindless_material,

            ctx: dvc_ctx,
            sto: rsc_sto,
            vpt: swc_ctx,
        })
    }

    pub fn render(&mut self, pkt: FrameRenderPacket) -> Result<FramePresentPacket> {
        let ctx = self.ctx.lock().eyre()?;
        let vpt = self.vpt.lock().eyre()?;

        let timeout = Duration::from_secs(1);

        // Wait until the commands have finished from the last time this frame was rendered
        ctx.wait_and_reset_fence(self.render_fence, timeout)?;

        // Acquire the next image from the swapchain
        let mut texture =
            vpt.acquire_next_present_texture(self.present_semaphore, timeout, &ctx.dev)?;

        // Record render commands
        self.cmd_encoder.begin_recording()?;

        self.cmd_encoder.transition_texture_layout(
            &mut texture.texture,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );

        // TODO: Perform render operations here

        self.cmd_encoder.transition_texture_layout(
            &mut texture.texture,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );

        let cmd = self.cmd_encoder.end_recording()?;

        // Submit render commands
        ctx.dev.submit(
            cmd,
            ctx.dev.get_graphics_queue(),
            &[self.present_semaphore],
            &[self.render_semaphore],
            self.render_fence,
        )?;

        Ok(FramePresentPacket { texture })
    }

    pub fn present(&self, pkt: FramePresentPacket) -> Result<PresentResult> {
        let vpt = self.vpt.lock().eyre()?;
        vpt.present(pkt.texture, self.render_semaphore)
    }
}

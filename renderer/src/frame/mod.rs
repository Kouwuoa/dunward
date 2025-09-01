pub(crate) mod packet;

pub(crate) use crate::viewport::PresentResult;

use crate::context::RenderContext;
use crate::context::commands::CommandEncoder;
use crate::frame::packet::{FramePresentPacket, FrameRenderPacket};
use crate::resources::material::Material;
use crate::resources::megabuffer::MegabufferExt;
use crate::resources::megabuffer::{AllocatedMegabufferRegion, Megabuffer};
use crate::resources::texture::{ColorTexture, DepthTexture, Texture};
use crate::storage::RenderStorage;
use crate::utils::GuardResultExt;
use crate::viewport::RenderViewport;
use ash::vk;
use color_eyre::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_FRAME_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_MATERIAL_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_OBJECT_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB

pub(crate) struct RenderFrame {
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

    ctx: Arc<Mutex<RenderContext>>,
    vpt: Arc<Mutex<RenderViewport>>,
    sto: Arc<Mutex<RenderStorage>>,
}

impl RenderFrame {
    pub fn new(
        ctx: Arc<Mutex<RenderContext>>,
        vpt: Arc<Mutex<RenderViewport>>,
        sto: Arc<Mutex<RenderStorage>>,
    ) -> Result<Self> {
        log::info!("Creating RenderFrame");

        let mut ctx_grd = ctx.lock().eyre()?;
        let mut vpt_grd = vpt.lock().eyre()?;
        let mut sto_grd = sto.lock().eyre()?;

        let target_size = ctx_grd.target.get_size();
        let draw_color_tex = ctx_grd.device.create_color_texture(
            target_size.width,
            target_size.height,
            None,
            true,
        )?;
        let draw_depth_tex = ctx_grd
            .device
            .create_depth_texture(target_size.width, target_size.height)?;

        let vertex_region = sto_grd
            .vertex_megabuffer
            .allocate_region(FRAME_VERTEX_BUFFER_SIZE)?;
        let index_region = sto_grd
            .index_megabuffer
            .allocate_region(FRAME_INDEX_BUFFER_SIZE)?;
        let per_frame_region = sto_grd
            .per_frame_megabuffer
            .allocate_region(FRAME_PER_FRAME_BUFFER_SIZE)?;
        let per_material_region = sto_grd
            .per_material_megabuffer
            .allocate_region(FRAME_PER_MATERIAL_BUFFER_SIZE)?;
        let per_object_region = sto_grd
            .per_object_megabuffer
            .allocate_region(FRAME_PER_OBJECT_BUFFER_SIZE)?;

        let present_semaphore = unsafe {
            ctx_grd
                .device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };
        let render_semaphore = unsafe {
            ctx_grd
                .device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };
        let render_fence = unsafe {
            ctx_grd.device.logical.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };

        let graphics_queue = ctx_grd.device.graphics_queue.clone();
        let cmd_encoder = ctx_grd.device.allocate_command_encoder(graphics_queue)?;

        let bindless_material = sto_grd.bindless_material_factory.create_material()?;

        drop(ctx_grd);
        drop(sto_grd);

        Ok(Self {
            draw_color_tex,
            draw_depth_tex,

            vertex_region,
            index_region,
            per_frame_region,
            per_material_region,
            per_object_region,

            swapchain_ready_sem: present_semaphore,
            render_finished_sem: render_semaphore,
            render_fence,

            cmd_encoder,
            bindless_material,

            ctx,
            sto,
        })
    }

    pub fn render(&self, pkt: FrameRenderPacket) -> Result<FramePresentPacket> {
        let ctx = self.ctx.lock().eyre()?;
        let vpt = self.vpt.lock().eyre()?;

        let timeout = Duration::from_secs(1);

        // Wait until the commands have finished from the last time this frame was rendered
        ctx.wait_and_reset_fence(self.render_fence, timeout)?;

        // Acquire the next image from the swapchain
        let image = vpt.acquire_next_present_image(self.present_semaphore, timeout)?;

        Ok(FramePresentPacket {
            image
        })
    }

    pub fn present(&self, pkt: FramePresentPacket) -> Result<PresentResult> {
        let vpt = self.vpt.lock().eyre()?;
        vpt.present(pkt.image, self.render_semaphore)
    }
}

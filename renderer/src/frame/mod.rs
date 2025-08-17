use std::sync::{Arc, LockResult, RwLock, RwLockWriteGuard};
use std::time::Duration;
use crate::context::RenderContext;
use crate::context::commands::CommandEncoder;
use crate::resources::image::Image;
use crate::resources::megabuffer::MegabufferExt;
use crate::resources::megabuffer::{AllocatedMegabufferRegion, Megabuffer};
use crate::storage::RenderStorage;
use ash::vk;
use color_eyre::Result;
use crate::context::target::RenderTarget;
use crate::frame::packet::{FramePresentPacket, FrameRenderPacket};

pub(crate) mod packet;

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_FRAME_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_MATERIAL_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_OBJECT_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB

pub(crate) struct RenderFrame {
    draw_color_image: Image,
    draw_depth_image: Image,

    vertex_region: AllocatedMegabufferRegion,
    index_region: AllocatedMegabufferRegion,
    per_frame_region: AllocatedMegabufferRegion,
    per_material_region: AllocatedMegabufferRegion,
    per_object_region: AllocatedMegabufferRegion,

    /// Signals when the swapchain is ready to present.
    present_semaphore: vk::Semaphore,
    /// Signals when rendering commands have been submitted to a queue.
    render_semaphore: vk::Semaphore,
    /// Signals when all rendering commands have finished execution.
    render_fence: vk::Fence,

    cmd_encoder: CommandEncoder,
    ctx: Arc<RwLock<RenderContext>>,
}

impl RenderFrame {
    pub fn new(ctx: Arc<RwLock<RenderContext>>, sto: &RenderStorage) -> Result<Self> {
        log::info!("Creating RenderFrame");

        let mut guard = ctx.write()?;
        let target_size = guard.target.as_ref().unwrap().get_size();

        let draw_color_image =
            guard.device
                .create_color_image(target_size.width, target_size.height, None, true)?;
        let draw_depth_image = guard
            .device
            .create_depth_image(target_size.width, target_size.height)?;

        let vertex_region = sto
            .vertex_megabuffer
            .allocate_region(FRAME_VERTEX_BUFFER_SIZE)?;
        let index_region = sto
            .index_megabuffer
            .allocate_region(FRAME_INDEX_BUFFER_SIZE)?;
        let per_frame_region = sto
            .per_frame_megabuffer
            .allocate_region(FRAME_PER_FRAME_BUFFER_SIZE)?;
        let per_material_region = sto
            .per_material_megabuffer
            .allocate_region(FRAME_PER_MATERIAL_BUFFER_SIZE)?;
        let per_object_region = sto
            .per_object_megabuffer
            .allocate_region(FRAME_PER_OBJECT_BUFFER_SIZE)?;

        let present_semaphore = unsafe {
            guard.device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };
        let render_semaphore = unsafe {
            guard.device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };
        let render_fence = unsafe {
            guard.device.logical.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };

        let graphics_queue = guard.device.graphics_queue.clone();
        let cmd_encoder = guard
            .device
            .allocate_command_encoder(graphics_queue)?;

        drop(guard);

        Ok(Self {
            draw_color_image,
            draw_depth_image,

            vertex_region,
            index_region,
            per_frame_region,
            per_material_region,
            per_object_region,

            present_semaphore,
            render_semaphore,
            render_fence,

            cmd_encoder,
            ctx,
        })
    }

    pub fn render(&self, pkt: FrameRenderPacket) -> Result<FramePresentPacket> {
        let ctx = self.ctx.read()?;
        // Wait until the commands have finished from the last time this frame was rendered
        let timeout = Duration::from_secs(1);
        ctx.wait_and_reset_fence(self.render_fence, timeout.as_nanos() as u64)?;

        Ok(())
    }

    pub fn present(&self, pkt: FramePresentPacket) -> Result<()> {
        let target = self.ctx.read()
            .map_err(|e| color_eyre::eyre::eyre!("Failed to read context: {}", e))?
            .target
            .as_ref()
            .ok_or_else(|| color_eyre::eyre::eyre!("Render target was not set"))?;

        let swapchain_image_index = pkt.swapchain_image_index;
        let present_info = vk::PresentInfoKHR {
            p_swapchains: &target.swapchain.swapchain,
            swapchain_count: 1,
            p_wait_semaphores: &self.render_semaphore, // Wait until rendering is done before presenting
            wait_semaphore_count: 1,
            p_image_indices: &swapchain_image_index,
            ..Default::default()
        };
        unsafe {
            self.ctx.swapchain
                .swapchain_loader
                .queue_present(ctx.context.present_queue, &present_info)?;
        }
        Ok(())
    }
}


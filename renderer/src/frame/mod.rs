use crate::context::RenderContext;
use crate::context::commands::CommandEncoder;
use crate::context::target::RenderTarget;
use crate::frame::packet::{FramePresentPacket, FrameRenderPacket};
use crate::resources::megabuffer::MegabufferExt;
use crate::resources::megabuffer::{AllocatedMegabufferRegion, Megabuffer};
use crate::resources::texture::{ColorTexture, DepthTexture, Texture};
use crate::storage::RenderStorage;
use crate::utils::GuardResultExt;
use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use std::sync::{Arc, LockResult, Mutex, RwLock, RwLockWriteGuard};
use std::time::Duration;

pub(crate) mod packet;

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_FRAME_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_MATERIAL_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_OBJECT_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB

pub(crate) enum PresentResult {
    Success,
    ResizeRequested,
}

pub(crate) struct RenderFrame {
    draw_color_tex: ColorTexture,
    draw_depth_tex: DepthTexture,

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
    ctx: Arc<Mutex<RenderContext>>,
}

impl RenderFrame {
    pub fn new(ctx: Arc<Mutex<RenderContext>>, sto: &RenderStorage) -> Result<Self> {
        log::info!("Creating RenderFrame");

        let mut guard = ctx.lock().eyre()?;
        let target_size = guard.target.as_ref().unwrap().get_size();

        let draw_color_tex =
            guard
                .device
                .create_color_texture(target_size.width, target_size.height, None, true)?;
        let draw_depth_tex = guard
            .device
            .create_depth_texture(target_size.width, target_size.height)?;

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
            guard
                .device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };
        let render_semaphore = unsafe {
            guard
                .device
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
        let cmd_encoder = guard.device.allocate_command_encoder(graphics_queue)?;

        drop(guard);

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
            ctx,
        })
    }

    pub fn render(&self, pkt: FrameRenderPacket) -> Result<FramePresentPacket> {
        let ctx = self.ctx.lock().eyre()?;
        let timeout = Duration::from_secs(1);

        // Wait until the commands have finished from the last time this frame was rendered
        ctx.wait_and_reset_fence(self.render_fence, timeout)?;

        // Acquire the next image from the swapchain
        let (swapchain_image, swapchain_image_index, swapchain_image_extent) =
            ctx.acquire_next_swapchain_image(self.present_semaphore, timeout)?;

        Ok(FramePresentPacket {
            swapchain_image_index,
        })
    }

    pub fn present(&self, pkt: FramePresentPacket) -> Result<PresentResult> {
        let ctx = self.ctx.lock().eyre()?;

        let target = ctx
            .target
            .as_ref()
            .ok_or_eyre("Render target was not set")?;
        let swapchain_image_index = pkt.swapchain_image_index;
        let present_info = vk::PresentInfoKHR {
            p_swapchains: &target.swapchain.swapchain,
            swapchain_count: 1,
            p_wait_semaphores: &self.render_semaphore, // Wait until rendering is done before presenting
            wait_semaphore_count: 1,
            p_image_indices: &swapchain_image_index,
            ..Default::default()
        };

        let present_queue = ctx.device.graphics_queue.as_ref();
        assert!(present_queue.family.supports_present()); // Ensure the queue supports presentation

        let present_result = unsafe {
            target
                .swapchain
                .swapchain_loader
                .queue_present(present_queue.handle, &present_info)
        };
        match present_result {
            Ok(true) => Ok(PresentResult::ResizeRequested),
            Ok(false) => Ok(PresentResult::Success),
            Err(err_code) => Err(eyre!(
                "Failed to present frame. VkResult error code: {}",
                err_code
            )),
        }
    }
}

use crate::context::RenderContext;
use crate::resources::megabuffer::Megabuffer;
use crate::resources::megabuffer::MegabufferExt;
use crate::storage::RenderStorage;
use ash::vk;
use ash::vk::Image;
use color_eyre::Result;

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB

pub struct RenderFrame {
    draw_color_image: Image,
    draw_depth_image: Image,
    vertex_buffer: Megabuffer,
    index_buffer: Megabuffer,

    /// Signals when the swapchain is ready to present.
    present_semaphore: vk::Semaphore,

    /// Signals when rendering commands have been submitted to a queue.
    render_semaphore: vk::Semaphore,

    /// Signals when all rendering commands have finished execution.
    render_fence: vk::Fence,
}

impl RenderFrame {
    pub fn new(ctx: &RenderContext, sto: &RenderStorage) -> Result<Self> {
        let target_size = ctx.target.as_ref().unwrap().get_size();

        let draw_color_image =
            ctx.device
                .create_color_image(target_size.width, target_size.height, None, true)?;
        let draw_depth_image = ctx
            .device
            .create_depth_image(target_size.width, target_size.height)?;

        let vertex_subbuffer = sto
            .vertex_megabuffer
            .allocate_region(FRAME_VERTEX_BUFFER_SIZE)?;
        let index_subbuffer = sto
            .index_megabuffer
            .all(FRAME_INDEX_BUFFER_SIZE)?;

        let present_semaphore = unsafe {
            dev_ctx
                .device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };
        let render_semaphore = unsafe {
            dev_ctx
                .device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };
        let render_fence = unsafe {
            dev_ctx.device.logical.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };

        Ok(Self {
            draw_color_image,
            draw_depth_image,
            vertex_subbuffer,
            index_subbuffer,
            present_semaphore,
            render_semaphore,
            render_fence,
        })
    }
}

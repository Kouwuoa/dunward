use crate::context::RenderContext;
use crate::context::commands::CommandEncoder;
use crate::resources::image::Image;
use crate::resources::megabuffer::MegabufferExt;
use crate::resources::megabuffer::{AllocatedMegabufferRegion, Megabuffer};
use crate::storage::RenderStorage;
use ash::vk;
use color_eyre::Result;

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_FRAME_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_MATERIAL_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_OBJECT_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB

pub struct RenderFrame {
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
}

impl RenderFrame {
    pub fn new(ctx: &mut RenderContext, sto: &RenderStorage) -> Result<Self> {
        log::info!("Creating RenderFrame");

        let target_size = ctx.target.as_ref().unwrap().get_size();

        let draw_color_image =
            ctx.device
                .create_color_image(target_size.width, target_size.height, None, true)?;
        let draw_depth_image = ctx
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
            ctx.device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };
        let render_semaphore = unsafe {
            ctx.device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        };
        let render_fence = unsafe {
            ctx.device.logical.create_fence(
                &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )?
        };

        let cmd_encoder = ctx
            .device
            .allocate_command_encoder(ctx.device.graphics_queue.clone())?;

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
        })
    }
}

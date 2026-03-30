use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::contexts::frame_context::packet::{FramePresentPacket, FrameRenderPacket};
use crate::renderer::contexts::swapchain_context::SwapchainContext;
use crate::renderer::subsystems::command_subsystem::CommandSubsystem;
use crate::renderer::subsystems::command_subsystem::command_recorder::{CommandRecorder, Idle};
use crate::renderer::subsystems::command_subsystem::command_recorder_allocator::CommandRecorderAllocatorExt;
use crate::renderer::subsystems::resource_subsystem::ResourceSubsystem;
use crate::renderer::subsystems::resource_subsystem::resource_types::material::Material;
use crate::renderer::subsystems::resource_subsystem::resource_types::megabuffer::{
    AllocatedMegabufferRegion, MegabufferExt,
};
use crate::renderer::subsystems::resource_subsystem::resource_types::texture::StorageTexture;
use ash::vk;
use color_eyre::Result;

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_FRAME_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_MATERIAL_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_OBJECT_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB

pub(super) struct FrameGeometryStage {
    recorder: Option<CommandRecorder<Idle>>,

    vertex_region: AllocatedMegabufferRegion,
    index_region: AllocatedMegabufferRegion,
    per_frame_region: AllocatedMegabufferRegion,
    per_material_region: AllocatedMegabufferRegion,
    per_object_region: AllocatedMegabufferRegion,

    finished_semaphore: vk::Semaphore,
    finished_fence: vk::Fence,

    material: Material,
}

impl FrameGeometryStage {
    pub fn new(
        dvc_ctx: &DeviceContext,
        cmd_sys: &mut CommandSubsystem,
        rsc_sys: &mut ResourceSubsystem,
    ) -> Result<Self> {
        let graphics_queue = dvc_ctx.get_graphics_queue();
        let recorder = Some(
            cmd_sys
                .command_recorder_allocator
                .allocate(graphics_queue)?,
        );

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

        let finished_semaphore = dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let finished_fence = dvc_ctx.create_vk_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
        )?;

        let material = rsc_sys
            .resource_store
            .graphics_material_factory
            .create_material()?;

        Ok(Self {
            recorder,
            vertex_region,
            index_region,
            per_frame_region,
            per_material_region,
            per_object_region,
            finished_semaphore,
            finished_fence,
            material,
        })
    }
}

//! Frame geometry rendering stage.
//!
//! Owns per-frame megabuffer regions (vertex, index, per-frame uniform, per-material, per-object)
//! and records rasterization commands for 3D meshes.

use crate::commands::allocator::{CommandRecorderAllocator, CommandRecorderAllocatorExt};
use crate::commands::recorder::{CommandRecorder, Idle};
use crate::core::DeviceContext;
use crate::resources::megabuffer::{AllocatedMegabufferRegion, MegabufferExt};
use crate::resources::store::ResourceStore;
use ash::vk;
use color_eyre::Result;

const FRAME_VERTEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_INDEX_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_FRAME_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_MATERIAL_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB
const FRAME_PER_OBJECT_BUFFER_SIZE: u64 = 1024 * 1024; // 1 MB

pub struct FrameGeometryStage {
    #[allow(dead_code)]
    recorder: Option<CommandRecorder<Idle>>,

    #[allow(dead_code)]
    vertex_region: AllocatedMegabufferRegion,
    #[allow(dead_code)]
    index_region: AllocatedMegabufferRegion,
    #[allow(dead_code)]
    per_frame_region: AllocatedMegabufferRegion,
    #[allow(dead_code)]
    per_material_region: AllocatedMegabufferRegion,
    #[allow(dead_code)]
    per_object_region: AllocatedMegabufferRegion,

    #[allow(dead_code)]
    finished_fence: vk::Fence,
}

impl FrameGeometryStage {
    #[allow(dead_code)]
    const TIMELINE_SEM_SIGNAL_VALUE: u64 = 1; // TODO: ignored for now

    pub fn new(
        dvc_ctx: &DeviceContext,
        cmd_allocator: &mut CommandRecorderAllocator,
        resource_store: &ResourceStore,
    ) -> Result<Self> {
        let graphics_queue = dvc_ctx.get_graphics_queue();
        let recorder = Some(cmd_allocator.allocate(graphics_queue)?);

        let vertex_region = resource_store
            .vertex_megabuffer
            .allocate_region(FRAME_VERTEX_BUFFER_SIZE)?;
        let index_region = resource_store
            .index_megabuffer
            .allocate_region(FRAME_INDEX_BUFFER_SIZE)?;
        let per_frame_region = resource_store
            .per_frame_megabuffer
            .allocate_region(FRAME_PER_FRAME_BUFFER_SIZE)?;
        let per_material_region = resource_store
            .per_material_megabuffer
            .allocate_region(FRAME_PER_MATERIAL_BUFFER_SIZE)?;
        let per_object_region = resource_store
            .per_object_megabuffer
            .allocate_region(FRAME_PER_OBJECT_BUFFER_SIZE)?;

        let finished_fence = dvc_ctx.create_vk_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
        )?;

        Ok(Self {
            recorder,
            vertex_region,
            index_region,
            per_frame_region,
            per_material_region,
            per_object_region,
            finished_fence,
        })
    }
}

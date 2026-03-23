use crate::renderer::subsystems::command_subsystem::command_recorder::{CommandRecorder, Idle};
use crate::renderer::subsystems::resource_subsystem::resource_types::megabuffer::AllocatedMegabufferRegion;
use ash::vk;
use color_eyre::Result;

pub(super) struct FrameGeometryStage {
    recorder: Option<CommandRecorder<Idle>>,

    vertex_region: AllocatedMegabufferRegion,
    index_region: AllocatedMegabufferRegion,
    per_frame_region: AllocatedMegabufferRegion,
    per_material_region: AllocatedMegabufferRegion,
    per_object_region: AllocatedMegabufferRegion,

    finished_semaphore: vk::Semaphore,
    finished_fence: vk::Fence,
}

impl FrameGeometryStage {
    pub fn new() -> Result<Self> {

    }
}

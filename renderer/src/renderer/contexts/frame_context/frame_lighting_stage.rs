use crate::renderer::contexts::swapchain_context::SwapchainContext;
use crate::renderer::subsystems::command_subsystem::command_recorder::{CommandRecorder, Idle};
use crate::renderer::subsystems::resource_subsystem::resource_types::material::Material;
use crate::renderer::subsystems::resource_subsystem::resource_types::texture::StorageTexture;
use ash::vk;
use color_eyre::Result;
use crate::renderer::subsystems::resource_subsystem::ResourceSubsystem;

pub(super) struct FrameLightingStage {
    recorder: Option<CommandRecorder<Idle>>,

    target_tex: StorageTexture,
    target_tex_needs_update: bool,

    finished_semaphore: vk::Semaphore,
    finished_fence: vk::Fence,

    material: Material,
}

impl FrameLightingStage {
    pub fn new(swc_ctx: &SwapchainContext, rsc_sys: &mut ResourceSubsystem) -> Result<Self> {
        let swc_size = swc_ctx.get_size();
        let target_tex = rsc_sys.resource_factory.create_storage_texture(
            swc_size.width,
            swc_size.height,
            true,
        )?;
        let material = rsc_sys
            .resource_store
            .bindless_material_factory
            .create_material()?;
    }
}

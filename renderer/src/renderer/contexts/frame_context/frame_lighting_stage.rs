use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::contexts::swapchain_context::SwapchainContext;
use crate::renderer::subsystems::command_subsystem::CommandSubsystem;
use crate::renderer::subsystems::command_subsystem::command_recorder::{CommandRecorder, Idle};
use crate::renderer::subsystems::command_subsystem::command_recorder_allocator::CommandRecorderAllocatorExt;
use crate::renderer::subsystems::resource_subsystem::ResourceSubsystem;
use crate::renderer::subsystems::resource_subsystem::resource_types::material::Material;
use crate::renderer::subsystems::resource_subsystem::resource_types::texture::StorageTexture;
use ash::vk;
use color_eyre::Result;

pub(super) struct FrameLightingStage {
    recorder: Option<CommandRecorder<Idle>>,

    target_tex: StorageTexture,
    target_tex_needs_update: bool,

    finished_semaphore: vk::Semaphore,
    finished_fence: vk::Fence,

    material: Material,
}

impl FrameLightingStage {
    pub fn new(
        dvc_ctx: &DeviceContext,
        swc_ctx: &SwapchainContext,
        cmd_sys: &mut CommandSubsystem,
        rsc_sys: &mut ResourceSubsystem,
    ) -> Result<Self> {
        let compute_queue = dvc_ctx.get_compute_queue();
        let recorder = Some(cmd_sys.command_recorder_allocator.allocate(compute_queue)?);

        let swc_size = swc_ctx.get_size();
        let target_tex = rsc_sys.resource_factory.create_storage_texture(
            swc_size.width,
            swc_size.height,
            true,
        )?;

        let finished_semaphore = dvc_ctx.create_vk_semaphore(&vk::SemaphoreCreateInfo::default())?;
        let finished_fence = dvc_ctx.create_vk_fence(
            &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
        )?;

        let material = rsc_sys
            .resource_store
            .compute_material_factory
            .create_material()?;

        Ok(Self {
            recorder,
            target_tex,
            target_tex_needs_update: true,
            finished_semaphore,
            finished_fence,
            material,
        })
    }
}

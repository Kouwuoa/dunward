use crate::renderer::{
    contexts::device_context::DeviceContext,
    contexts::swapchain_context::SwapchainContext,
    subsystems::{
        command_subsystem::CommandSubsystem,
        command_subsystem::transfer_command_recorder::TransferCommandRecorder,
        resource_subsystem::resource_binder::ResourceBinder,
        resource_subsystem::resource_store::ResourceStore,
        resource_subsystem::resource_types::megabuffer::MegabufferExt,
    },
};
use color_eyre::eyre::Result;
use std::sync::{Arc, Mutex};

mod resource_binder;
mod resource_factory;
pub(crate) mod resource_store;
mod resource_types;

pub(crate) struct ResourceSubsystem {
    resource_store: ResourceStore,
    resource_binder: ResourceBinder,
    memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
    transfer_command_recorder: Arc<TransferCommandRecorder>,
    device: Arc<ash::Device>,
}

impl ResourceSubsystem {
    pub fn new(
        dvc_ctx: &DeviceContext,
        swc_ctx: &SwapchainContext,
        cmd_sys: &CommandSubsystem,
    ) -> Result<Self> {
        let resource_store = ResourceStore::new();
        let resource_binder = ResourceBinder::new();
        let memory_allocator = unsafe {
            vk_mem::Allocator::new(vk_mem::AllocatorCreateInfo::new(
                &dvc_ctx.instance_handle(),
                &dvc_ctx.logical_device_handle(),
                dvc_ctx.physical_device_handle(),
            ))?
        };
        let transfer_command_recorder = cmd_sys.transfer_command_recorder.clone();

        let mut result = Self {
            resource_store,
            resource_binder,
            memory_allocator: Arc::new(Mutex::new(memory_allocator)),
            transfer_command_recorder,
            device: dvc_ctx.logical_device_handle(),
        };

        let resource_store = &mut result.resource_store;
        resource_store.init(&result)?;

        let nearest_sampler = factory.create_vk_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::NEAREST)
                .min_filter(vk::Filter::NEAREST)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT),
        )?;

        Ok(result)
    }
}

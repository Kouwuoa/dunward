use crate::renderer::subsystems::resource_subsystem::resource_factory::ResourceFactory;
use crate::renderer::{
    contexts::device_context::DeviceContext,
    subsystems::{
        command_subsystem::CommandSubsystem,
        command_subsystem::transfer_command_recorder::TransferCommandRecorder,
        resource_subsystem::resource_updater::ResourceUpdater,
        resource_subsystem::resource_store::ResourceStore,
        resource_subsystem::resource_types::megabuffer::MegabufferExt,
    },
};
use ash::vk;
use color_eyre::eyre::Result;
use std::sync::{Arc, Mutex};

pub(crate) mod resource_updater;
pub(crate) mod resource_descriptors;
pub(crate) mod resource_factory;
pub(crate) mod resource_store;
pub(crate) mod resource_types;

pub(crate) struct ResourceSubsystem {
    pub resource_store: ResourceStore,
    pub resource_factory: ResourceFactory,
    memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
    transfer_command_recorder: Arc<TransferCommandRecorder>,
    device: Arc<ash::Device>,
}

impl ResourceSubsystem {
    pub fn new(dvc_ctx: &DeviceContext, cmd_sys: &CommandSubsystem) -> Result<Self> {
        let memory_allocator = Arc::new(Mutex::new(unsafe {
            vk_mem::Allocator::new(vk_mem::AllocatorCreateInfo::new(
                &dvc_ctx.instance_handle(),
                &dvc_ctx.logical_device_handle(),
                dvc_ctx.physical_device_handle(),
            ))?
        }));
        let transfer_command_recorder = cmd_sys.transfer_command_recorder.clone();
        let resource_factory = ResourceFactory::new(
            memory_allocator.clone(),
            transfer_command_recorder.clone(),
            dvc_ctx.logical_device_handle(),
        )?;
        let mut resource_store = ResourceStore::new(&resource_factory)?;
        let nearest_sampler = dvc_ctx.create_vk_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::NEAREST)
                .min_filter(vk::Filter::NEAREST)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT),
        )?;
        resource_store.add_sampler(nearest_sampler);

        Ok(Self {
            resource_store,
            resource_factory,
            memory_allocator,
            transfer_command_recorder,
            device: dvc_ctx.logical_device_handle(),
        })
    }

    pub fn get_memory_allocator(&self) -> Arc<Mutex<vk_mem::Allocator>> {
        self.memory_allocator.clone()
    }
}

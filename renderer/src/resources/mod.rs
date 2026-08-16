//! GPU Resource Subsystem: Memory Allocations, Buffers, Textures, and Descriptors.
//!
//! Exposes [`ResourceSubsystem`], which coordinates GPU memory allocation via [`vk_mem::Allocator`],
//! manages long-lived assets in [`ResourceStore`], and creates GPU primitives via [`ResourceFactory`].

pub mod buffer;
pub mod descriptors;
pub mod factory;
pub mod megabuffer;
pub mod store;
pub mod texture;
pub mod updater;

pub use buffer::Buffer;
pub use descriptors::DescriptorAllocator;
pub use factory::ResourceFactory;
pub use megabuffer::{AllocatedMegabufferRegion, Megabuffer, MegabufferExt};
pub use store::ResourceStore;
pub use texture::{
    ColorTexture, DepthTexture, StorageTexture, Texture, TextureAccess, TextureQueueState,
};
pub use updater::{ResourceUpdateBuilder, ResourceUpdater};

use crate::commands::CommandSubsystem;
use crate::commands::transfer::TransferCommandRecorder;
use crate::core::DeviceContext;
use ash::vk;
use color_eyre::eyre::Result;
use std::sync::{Arc, Mutex};

#[derive(PartialEq)]
pub enum ResourceType {
    UniformBuffer,
    StorageBuffer,
    StorageImage,
    Sampler,
    SampledImage,
}

impl ResourceType {
    pub const ALL: &'static [Self] = &[
        Self::UniformBuffer,
        Self::StorageBuffer,
        Self::StorageImage,
        Self::Sampler,
        Self::SampledImage,
    ];

    pub fn descriptor_type(&self) -> vk::DescriptorType {
        match self {
            Self::UniformBuffer => vk::DescriptorType::UNIFORM_BUFFER,
            Self::StorageBuffer => vk::DescriptorType::STORAGE_BUFFER,
            Self::StorageImage => vk::DescriptorType::STORAGE_IMAGE,
            Self::Sampler => vk::DescriptorType::SAMPLER,
            Self::SampledImage => vk::DescriptorType::SAMPLED_IMAGE,
        }
    }

    pub fn descriptor_binding_flags(&self) -> vk::DescriptorBindingFlags {
        match self {
            Self::UniformBuffer => {
                vk::DescriptorBindingFlags::PARTIALLY_BOUND
                    | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            }
            Self::StorageBuffer => {
                vk::DescriptorBindingFlags::PARTIALLY_BOUND
                    | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            }
            Self::StorageImage => {
                vk::DescriptorBindingFlags::PARTIALLY_BOUND
                    | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            }
            Self::Sampler => {
                vk::DescriptorBindingFlags::PARTIALLY_BOUND
                    | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
            }
            Self::SampledImage => {
                vk::DescriptorBindingFlags::PARTIALLY_BOUND
                    | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND
                    | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
            }
        }
    }

    pub fn descriptor_pool_count(&self) -> u32 {
        match self {
            Self::UniformBuffer => 16,
            Self::StorageBuffer => 16,
            Self::StorageImage => 16,
            Self::Sampler => 16,
            Self::SampledImage => 16,
        }
    }
}

pub struct ResourceSubsystem {
    pub resource_store: ResourceStore,
    pub resource_factory: ResourceFactory,
    memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
    #[allow(dead_code)]
    transfer_command_recorder: Arc<TransferCommandRecorder>,
    #[allow(dead_code)]
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

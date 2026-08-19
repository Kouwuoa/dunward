//! GPU Resource Subsystem: Memory Allocations, Buffers, Textures, and Descriptors.
//!
//! Exposes [`ResourceStore`] for managing long-lived GPU assets, [`ResourceFactory`]
//! for constructing GPU memory allocations and textures, and [`Megabuffer`] for pooled sub-allocations.

pub(crate) mod buffer;
pub(crate) mod descriptors;
pub(crate) mod factory;
pub(crate) mod megabuffer;
pub(crate) mod store;
pub(crate) mod texture;
pub(crate) mod updater;

use std::sync::{Arc, Mutex};

use ash::vk;
use color_eyre::eyre::Result;

use crate::core::DeviceContext;

/// Helper function to instantiate the `vk_mem::Allocator` instance.
pub(crate) fn create_memory_allocator(
    dvc_ctx: &DeviceContext,
) -> Result<Arc<Mutex<vk_mem::Allocator>>> {
    Ok(Arc::new(Mutex::new(unsafe {
        vk_mem::Allocator::new(vk_mem::AllocatorCreateInfo::new(
            &dvc_ctx.instance_handle(),
            &dvc_ctx.logical_device_handle(),
            dvc_ctx.physical_device_handle(),
        ))?
    })))
}

#[derive(PartialEq)]
pub(crate) enum ResourceType {
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

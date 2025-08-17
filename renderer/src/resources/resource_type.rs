use ash::vk;

const UNIFORM_BUFFER_DESCRIPTOR_COUNT: u32 = 1;
const STORAGE_BUFFER_DESCRIPTOR_COUNT: u32 = 1;
const STORAGE_IMAGE_DESCRIPTOR_COUNT: u32 = 1;
const SAMPLER_DESCRIPTOR_COUNT: u32 = 16;
const SAMPLED_IMAGE_DESCRIPTOR_COUNT: u32 = 1024;

#[derive(PartialEq)]
pub enum RenderResourceType {
    UniformBuffer,
    StorageBuffer,
    StorageImage,
    Sampler,
    SampledImage,
}

impl RenderResourceType {
    const ALL: &'static [Self] = &[
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

    pub fn descriptor_count(&self) -> u32 {
        match self {
            Self::UniformBuffer => UNIFORM_BUFFER_DESCRIPTOR_COUNT,
            Self::StorageBuffer => STORAGE_BUFFER_DESCRIPTOR_COUNT,
            Self::StorageImage => STORAGE_IMAGE_DESCRIPTOR_COUNT,
            Self::Sampler => SAMPLER_DESCRIPTOR_COUNT,
            Self::SampledImage => SAMPLED_IMAGE_DESCRIPTOR_COUNT,
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

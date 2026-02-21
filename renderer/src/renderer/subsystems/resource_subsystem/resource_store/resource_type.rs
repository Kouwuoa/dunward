use ash::vk;

#[derive(PartialEq)]
pub enum ResourceType {
    UniformBuffer,
    StorageBuffer,
    StorageImage,
    Sampler,
    SampledImage,
}

impl ResourceType {
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

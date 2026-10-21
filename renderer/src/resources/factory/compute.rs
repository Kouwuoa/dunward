use crate::{
    material::{
        ComputeMaterialFactoryBuilder, MaterialFactory, shader::ComputeShader,
        shader_data::PerDrawData,
    },
    resources::{ResourceType, descriptor::DescriptorSetLayoutBuilder},
};

use super::ResourceFactory;

use ash::vk;
use color_eyre::Result;
use shaderpack::ShaderId;

impl ResourceFactory {
    pub fn create_compute_material_factory(&self) -> Result<MaterialFactory> {
        let descriptor_set_layout = self.create_compute_descriptor_set_layout()?;
        let pipeline_layout = self.create_compute_pipeline_layout(descriptor_set_layout)?;
        let compute_shader = ComputeShader::new(ShaderId::TestPattern, self.gpu.raw_logical())?;
        ComputeMaterialFactoryBuilder::new(
            self.gpu.raw_logical(),
            self.descriptor_allocator.clone(),
        )
        .with_shader(compute_shader)
        .with_pipeline_layout(pipeline_layout)
        .with_descriptor_set_layout(descriptor_set_layout)
        .build()
    }
    fn create_compute_descriptor_set_layout(&self) -> Result<vk::DescriptorSetLayout> {
        DescriptorSetLayoutBuilder::new()
            .add_binding(
                // Image to render to
                0,
                ResourceType::StorageImage.descriptor_type(),
                1,
                vk::ShaderStageFlags::COMPUTE,
                ResourceType::StorageImage.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Per-frame
                1,
                ResourceType::UniformBuffer.descriptor_type(),
                1,
                vk::ShaderStageFlags::COMPUTE,
                ResourceType::UniformBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Per-material
                2,
                ResourceType::StorageBuffer.descriptor_type(),
                1,
                vk::ShaderStageFlags::COMPUTE,
                ResourceType::StorageBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Per-object
                3,
                ResourceType::StorageBuffer.descriptor_type(),
                1,
                vk::ShaderStageFlags::COMPUTE,
                ResourceType::StorageBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Samplers
                4,
                ResourceType::Sampler.descriptor_type(),
                4,
                vk::ShaderStageFlags::COMPUTE,
                ResourceType::Sampler.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Sampled Textures
                5,
                ResourceType::SampledImage.descriptor_type(),
                4,
                vk::ShaderStageFlags::COMPUTE,
                ResourceType::SampledImage.descriptor_binding_flags(),
                None,
            )
            .build(
                vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
                &self.gpu.raw_logical(),
            )
    }

    fn create_compute_pipeline_layout(
        &self,
        bindless_descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<vk::PipelineLayout> {
        let push_constant_size = size_of::<PerDrawData>() as u32;
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .offset(0)
            .size(push_constant_size);
        let push_constant_ranges = [push_constant_range];

        let set_layouts = [bindless_descriptor_set_layout];
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_constant_ranges);

        let pipeline_layout = unsafe {
            self.gpu
                .raw_logical()
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };

        Ok(pipeline_layout)
    }
}

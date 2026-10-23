use crate::{
    material::{GraphicsMaterialFactoryBuilder, MaterialFactory, shader::GraphicsShader},
    resources::{ResourceType, descriptor::DescriptorSetLayoutBuilder},
};

use super::ResourceFactory;

use ash::vk;
use color_eyre::Result;
use shaderpack::ShaderId;

impl ResourceFactory {
    pub fn create_graphics_material_factory(&self) -> Result<MaterialFactory> {
        let descriptor_set_layout = self.create_graphics_descriptor_set_layout()?;
        let pipeline_layout = self.create_graphics_pipeline_layout(descriptor_set_layout)?;
        let graphics_shader = GraphicsShader::new(
            ShaderId::DefaultVert,
            ShaderId::DefaultFrag,
            self.gpu.raw_logical(),
        )?;
        GraphicsMaterialFactoryBuilder::new(
            self.gpu.raw_logical(),
            self.descriptor_allocator.clone(),
        )
        .with_shader(graphics_shader)
        .with_pipeline_layout(pipeline_layout)
    }

    fn create_graphics_pipeline_layout(&self, descriptor_set_layout: !) -> _ {
        todo!()
    }

    fn create_graphics_descriptor_set_layout(&self) -> Result<vk::DescriptorSetLayout> {
        DescriptorSetLayoutBuilder::new()
            .add_binding(
                // Per-frame
                0,
                ResourceType::UniformBuffer.descriptor_type(),
                1,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ResourceType::UniformBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Per-material
                1,
                ResourceType::StorageBuffer.descriptor_type(),
                1,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ResourceType::StorageBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Per-object
                2,
                ResourceType::StorageBuffer.descriptor_type(),
                1,
                vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
                ResourceType::StorageBuffer.descriptor_binding_flags(),
                None,
            )
            .build(
                vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
                &self.gpu.raw_logical(),
            )
    }
}

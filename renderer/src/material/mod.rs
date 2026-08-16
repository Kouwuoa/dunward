//! Material abstractions, shaders, pipeline states, and per-material descriptor sets.
//!
//! A [`Material`] represents an instantiated shader pipeline ready for GPU resource binding
//! and push constant updates during rendering.

pub(crate) mod compute;
pub(crate) mod graphics;
pub(crate) mod shader;
pub(crate) mod shader_data;

pub(crate) use compute::ComputeMaterialFactoryBuilder;
pub(crate) use graphics::GraphicsMaterialFactoryBuilder;
pub(crate) use shader::{ComputeShader, GraphicsShader};
pub(crate) use shader_data::{PerDrawData, PerFrameData, PerMaterialData, PerObjectData, PerVertexData};

use std::sync::{Arc, Mutex};

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::eyre;

use crate::resources::descriptors::DescriptorAllocator;

/// You can think of a Material as a shader instance that you can bind resources and data to.
/// You only need to create a Material once, and then you can use it to render multiple objects.
/// You only need to switch the Material when you want to change the shader or pipeline.
pub(crate) struct Material {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline_bind_point: vk::PipelineBindPoint,
    pub descriptor_set: vk::DescriptorSet,
    device: Arc<ash::Device>,
}

impl Material {
    pub fn update_push_constants(&self, command_buffer: vk::CommandBuffer, data: &[u8]) {
        unsafe {
            self.device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::COMPUTE,
                0,
                data,
            );
        }
    }

    pub fn bind(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device.cmd_bind_pipeline(
                command_buffer,
                self.pipeline_bind_point,
                self.pipeline,
            );
            let descriptor_sets = [self.descriptor_set];
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                self.pipeline_bind_point,
                self.pipeline_layout,
                0,
                &descriptor_sets,
                &[],
            );
        }
    }
}

pub(crate) struct MaterialFactory {
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline_bind_point: vk::PipelineBindPoint,
    pub descriptor_set_layout: vk::DescriptorSetLayout,

    device: Arc<ash::Device>,
    descriptor_allocator: Arc<Mutex<DescriptorAllocator>>,
}

impl MaterialFactory {
    pub fn create_material(&'_ mut self) -> Result<Material> {
        let descriptor_set = self.allocate_descriptor_set()?;
        Ok(Material {
            pipeline: self.pipeline,
            pipeline_layout: self.pipeline_layout,
            pipeline_bind_point: self.pipeline_bind_point,
            descriptor_set,
            device: self.device.clone(),
        })
    }

    fn allocate_descriptor_set(&mut self) -> Result<vk::DescriptorSet> {
        self.descriptor_allocator
            .lock()
            .map_err(|e| eyre!(e.to_string()))?
            .allocate(self.descriptor_set_layout)
    }
}

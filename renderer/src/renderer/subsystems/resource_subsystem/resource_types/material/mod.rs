mod graphics;
mod compute;

pub use graphics::GraphicsMaterialFactoryBuilder;
pub use compute::ComputeMaterialFactoryBuilder;

use crate::renderer::subsystems::resource_subsystem::resource_descriptors::descriptor_allocator::DescriptorAllocator;
use ash::vk;
use color_eyre::eyre::{eyre, OptionExt};
use color_eyre::Result;
use std::sync::{Arc, Mutex};

/// You can think of a Material as a shader instance that you can bind resources and data to.
/// You only need to create a Material once, and then you can use it to render multiple objects.
/// You only need to switch the Material when you want to change the shader or pipeline.
pub struct Material {
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

    pub fn bind_pipeline(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            self.device
                .cmd_bind_pipeline(command_buffer, self.pipeline_bind_point, self.pipeline);
        }
    }

    pub fn bind_descriptor_sets(&self, command_buffer: vk::CommandBuffer) {
        let descriptor_sets = [self.descriptor_set];
        unsafe {
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

pub struct MaterialFactory {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    pipeline_bind_point: vk::PipelineBindPoint,
    descriptor_set_layout: vk::DescriptorSetLayout,

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


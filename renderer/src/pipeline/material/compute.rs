//! Compute material factory builder and compute pipeline creation.
//!
//! Configures compute shaders, pipeline layouts, and descriptor set layouts
//! for compute dispatch workloads.

use super::MaterialFactory;
use crate::pipeline::shader::ComputeShader;
use crate::resources::descriptors::DescriptorAllocator;
use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use std::ffi::CString;
use std::sync::{Arc, Mutex};

pub struct ComputeMaterialFactoryBuilder {
    shader: Option<ComputeShader>,
    pipeline_layout: Option<vk::PipelineLayout>,
    descriptor_set_layout: Option<vk::DescriptorSetLayout>,

    device: Arc<ash::Device>,
    descriptor_allocator: Arc<Mutex<DescriptorAllocator>>,
}

impl ComputeMaterialFactoryBuilder {
    pub fn new(
        device: Arc<ash::Device>,
        descriptor_allocator: Arc<Mutex<DescriptorAllocator>>,
    ) -> Self {
        Self {
            shader: None,
            pipeline_layout: None,
            descriptor_set_layout: None,
            device,
            descriptor_allocator,
        }
    }

    pub fn with_shader(mut self, shader: ComputeShader) -> Self {
        let _ = self.shader.replace(shader);
        self
    }

    pub fn with_pipeline_layout(mut self, layout: vk::PipelineLayout) -> Self {
        let _ = self.pipeline_layout.replace(layout);
        self
    }

    pub fn with_descriptor_set_layout(mut self, layout: vk::DescriptorSetLayout) -> Self {
        let _ = self.descriptor_set_layout.replace(layout);
        self
    }

    pub fn build(mut self) -> Result<MaterialFactory> {
        let shader = self
            .shader
            .take()
            .ok_or_eyre("No shader provided for ComputeMaterialBuilder")?;
        let pipeline_layout = self
            .pipeline_layout
            .take()
            .ok_or_eyre("No pipeline layout provided for ComputeMaterialBuilder")?;

        let descriptor_set_layout = self
            .descriptor_set_layout
            .take()
            .ok_or_eyre("No descriptor set layout provided for GraphicsMaterialBuilder")?;

        let name = CString::new("main")?;
        let stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader.comp_mod)
            .name(&name);

        let pipeline_info = vk::ComputePipelineCreateInfo::default()
            .layout(pipeline_layout)
            .stage(stage_info);
        let pipeline = unsafe {
            match self.device.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            ) {
                Ok(pipelines) => Ok(pipelines),
                Err(_) => Err(eyre!("Failed to create compute pipeline")),
            }
        }?[0];

        Ok(MaterialFactory {
            pipeline,
            pipeline_layout,
            pipeline_bind_point: vk::PipelineBindPoint::COMPUTE,
            descriptor_set_layout,
            device: self.device,
            descriptor_allocator: self.descriptor_allocator,
        })
    }
}

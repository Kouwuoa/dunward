//! GPU Resource Factory for constructing buffers, textures, and material factories.
//!
//! Exposes [`ResourceFactory`], providing creation methods for [`Megabuffer`],
//! [`Texture`], [`ColorTexture`], [`DepthTexture`], and [`StorageTexture`].

use std::sync::{Arc, Mutex};

use ash::vk;
use color_eyre::Result;
use shaderpack::ShaderId;

use super::ResourceType;
use super::texture::{ColorTexture, DepthTexture, StorageTexture, Texture};
use crate::commands::transfer::TransferCommandRecorder;
use crate::gpu::Gpu;
use crate::material::shader::ComputeShader;
use crate::material::shader_data::PerDrawData;
use crate::material::{ComputeMaterialFactoryBuilder, MaterialFactory};
use crate::resources::descriptor::DescriptorAllocator;
use crate::resources::megabuffer::Megabuffer;

/// `ResourceFactory` is responsible only for construction (`create_*` APIs)
/// such as buffers, textures, and materials as needed.
/// It does not own long-lived storage, caching, or lookup tables.
pub(crate) struct ResourceFactory {
    transfer_command_recorder: Arc<Mutex<TransferCommandRecorder>>,
    descriptor_allocator: Arc<Mutex<DescriptorAllocator>>,
    gpu: Arc<Gpu>,
}

impl ResourceFactory {
    pub fn new(
        gpu: Arc<Gpu>,
    ) -> Result<Self> {
        let transfer_command_recorder = Arc::new(Mutex::new(TransferCommandRecorder::new(&gpu)?));
        let descriptor_allocator =
            Arc::new(Mutex::new(DescriptorAllocator::new(gpu.raw_logical(), 1000)?));
        Ok(Self {
            transfer_command_recorder,
            descriptor_allocator,
            gpu,
        })
    }

    pub fn create_color_texture(
        &self,
        width: u32,
        height: u32,
        data: Option<&[u8]>,
        use_dedicated_memory: bool,
        usage: vk::ImageUsageFlags,
    ) -> Result<ColorTexture> {
        Texture::new_color_texture_from_bytes(
            width,
            height,
            data,
            use_dedicated_memory,
            usage,
            self.memory_allocator.clone(),
            self.device.clone(),
            &self.transfer_command_recorder,
        )
    }

    pub fn create_depth_texture(&self, width: u32, height: u32) -> Result<DepthTexture> {
        Texture::new_depth_texture(
            width,
            height,
            self.gpu.clone(),
        )
    }

    pub fn create_megabuffer(
        &self,
        size: u64,
        alignment: u64,
        buf_usage: vk::BufferUsageFlags,
    ) -> Result<Megabuffer> {
        Megabuffer::new(
            size,
            alignment,
            buf_usage,
            self.gpu.clone(),
        )
    }

    pub fn create_storage_texture(
        &self,
        width: u32,
        height: u32,
        use_dedicated_memory: bool,
    ) -> Result<StorageTexture> {
        Texture::new_storage_texture(
            width,
            height,
            use_dedicated_memory,
            self.memory_allocator.clone(),
            self.device.clone(),
        )
    }

    pub fn create_compute_material_factory(&self) -> Result<MaterialFactory> {
        let descriptor_set_layout = self.create_compute_descriptor_set_layout()?;
        let pipeline_layout = self.create_compute_pipeline_layout(descriptor_set_layout)?;
        let compute_shader = ComputeShader::new(ShaderId::TestPattern, self.device.clone())?;
        ComputeMaterialFactoryBuilder::new(self.device.clone(), self.descriptor_allocator.clone())
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
                &self.device,
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
            self.device
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };

        Ok(pipeline_layout)
    }
}

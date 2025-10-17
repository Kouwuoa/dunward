use crate::viewport::RenderViewport;
use crate::{
    context::RenderContext,
    context::desc_set_layout_builder::DescriptorSetLayoutBuilder,
    resources::{
        material::{GraphicsMaterialFactoryBuilder, MaterialFactory},
        megabuffer::Megabuffer,
        model::FullscreenQuad,
        resource_type::RenderResourceType,
        shader::GraphicsShader,
        texture::{ColorTexture, StorageTexture},
    },
};
use ash::vk;
use color_eyre::Result;
use gpu_descriptor::DescriptorAllocator;
use shader_data::PerDrawData;
use std::sync::{Arc, Mutex};

pub(crate) mod shader_data;

const VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 256; // 256 MB
const INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 64; // 64 MB
const PER_FRAME_BUFFER_SIZE: u64 = 16 * 1024 * 1024; // 16 MB
const PER_MATERIAL_BUFFER_SIZE: u64 = 16 * 1024 * 1024; // 16 MB
const PER_OBJECT_BUFFER_SIZE: u64 = 16 * 1024 * 1024; // 16 MB
const VERTEX_BUFFER_ALIGNMENT: u64 = 16;
const INDEX_BUFFER_ALIGNMENT: u64 = 4;
const STORAGE_BUFFER_ALIGNMENT: u64 = 16;
const UNIFORM_BUFFER_ALIGNMENT: u64 = 256;

pub(crate) struct RenderStorage {
    pub storage_textures: Vec<StorageTexture>,
    pub sampled_textures: Vec<ColorTexture>,
    pub samplers: Vec<vk::Sampler>,

    pub vertex_megabuffer: Megabuffer,
    pub index_megabuffer: Megabuffer,
    pub per_frame_megabuffer: Megabuffer,
    pub per_material_megabuffer: Megabuffer,
    pub per_object_megabuffer: Megabuffer,
    pub bindless_material_factory: MaterialFactory,

    pub fullscreen_quad: FullscreenQuad,
}

impl RenderStorage {
    pub fn new(ctx: &RenderContext, vpt: &RenderViewport) -> Result<Self> {
        log::info!("Creating RenderStorage");
        
        let device = &ctx.dev;

        let vertex_megabuffer = device.create_megabuffer(
            VERTEX_BUFFER_SIZE,
            VERTEX_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let index_megabuffer = device.create_megabuffer(
            INDEX_BUFFER_SIZE,
            INDEX_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let per_frame_megabuffer = device.create_megabuffer(
            PER_FRAME_BUFFER_SIZE,
            UNIFORM_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let per_material_megabuffer = device.create_megabuffer(
            PER_MATERIAL_BUFFER_SIZE,
            STORAGE_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let per_object_megabuffer = device.create_megabuffer(
            PER_OBJECT_BUFFER_SIZE,
            STORAGE_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let bindless_material_factory = Self::create_bindless_material_factory(
            device.logical.clone(),
            device.descriptor_allocator.clone(),
        )?;

        let fullscreen_quad = FullscreenQuad::new(
            &vertex_megabuffer,
            &index_megabuffer,
            vpt,
        )?;

        let mut samplers = Vec::new();
        samplers.push(unsafe {
            device.logical.create_sampler(
                &vk::SamplerCreateInfo::default()
                    .mag_filter(vk::Filter::NEAREST)
                    .min_filter(vk::Filter::NEAREST)
                    .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                    .address_mode_u(vk::SamplerAddressMode::REPEAT)
                    .address_mode_v(vk::SamplerAddressMode::REPEAT)
                    .address_mode_w(vk::SamplerAddressMode::REPEAT),
                None,
            )?
        });

        Ok(Self {
            storage_textures: Vec::new(),
            sampled_textures: Vec::new(),
            samplers,

            vertex_megabuffer,
            index_megabuffer,
            per_frame_megabuffer,
            per_material_megabuffer,
            per_object_megabuffer,
            bindless_material_factory,

            fullscreen_quad,
        })
    }

    fn create_bindless_material_factory(
        device: Arc<ash::Device>,
        descriptor_allocator: Arc<
            Mutex<DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>>,
        >,
    ) -> Result<MaterialFactory> {
        let bindless_descriptor_set_layout = Self::create_bindless_descriptor_set_layout(&device)?;
        let bindless_pipeline_layout =
            Self::create_bindless_pipeline_layout(bindless_descriptor_set_layout, &device)?;
        let default_shader = GraphicsShader::new("default", device.clone())?;
        GraphicsMaterialFactoryBuilder::new(device, descriptor_allocator)
            .with_shader(default_shader)
            .with_pipeline_layout(bindless_pipeline_layout)
            .with_descriptor_set_layout(bindless_descriptor_set_layout)
            .with_color_attachment_format(vk::Format::R8G8B8A8_SRGB)
            .with_depth_attachment_format(vk::Format::D32_SFLOAT)
            .build()
    }

    fn create_bindless_descriptor_set_layout(
        device: &ash::Device,
    ) -> Result<vk::DescriptorSetLayout> {
        DescriptorSetLayoutBuilder::new()
            .add_binding(
                // Per-frame
                0,
                RenderResourceType::UniformBuffer.descriptor_type(),
                RenderResourceType::UniformBuffer.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                RenderResourceType::UniformBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Per-material
                1,
                RenderResourceType::StorageBuffer.descriptor_type(),
                RenderResourceType::StorageBuffer.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                RenderResourceType::StorageBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Per-object
                2,
                RenderResourceType::StorageBuffer.descriptor_type(),
                RenderResourceType::StorageBuffer.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                RenderResourceType::StorageBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Samplers
                3,
                RenderResourceType::Sampler.descriptor_type(),
                RenderResourceType::Sampler.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                RenderResourceType::Sampler.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Textures
                4,
                RenderResourceType::SampledImage.descriptor_type(),
                RenderResourceType::SampledImage.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                RenderResourceType::SampledImage.descriptor_binding_flags(),
                None,
            )
            .build(
                vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
                device,
            )
    }

    fn create_bindless_pipeline_layout(
        bindless_descriptor_set_layout: vk::DescriptorSetLayout,
        device: &ash::Device,
    ) -> Result<vk::PipelineLayout> {
        let push_constant_size = size_of::<PerDrawData>() as u32;
        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::ALL)
            .offset(0)
            .size(push_constant_size);
        let push_constant_ranges = [push_constant_range];

        let set_layouts = [bindless_descriptor_set_layout];
        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_constant_ranges);

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_create_info, None)? };

        Ok(pipeline_layout)
    }
}

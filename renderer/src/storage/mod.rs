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

        let bindless_material_factory = device.create_bindless_material_factory()?;

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
}

use crate::contexts::device_context::DeviceContext;
use crate::contexts::swapchain_context::SwapchainContext;
use ash::vk;
use color_eyre::Result;
use material::{GraphicsMaterialFactoryBuilder, MaterialFactory};
use megabuffer::Megabuffer;
use model::FullscreenQuad;
use resource_type::ResourceType;
use shader::GraphicsShader;
use std::sync::{Arc, Mutex};
use texture::{ColorTexture, StorageTexture};

pub(crate) mod buffer;
pub(crate) mod material;
pub(crate) mod megabuffer;
pub(crate) mod mesh;
pub(crate) mod model;
pub(crate) mod resource_type;
pub(crate) mod shader;
pub(crate) mod shader_data;
pub(crate) mod texture;
pub(crate) mod vertex;

const VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 256; // 256 MB
const INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 64; // 64 MB
const PER_FRAME_BUFFER_SIZE: u64 = 16 * 1024 * 1024; // 16 MB
const PER_MATERIAL_BUFFER_SIZE: u64 = 16 * 1024 * 1024; // 16 MB
const PER_OBJECT_BUFFER_SIZE: u64 = 16 * 1024 * 1024; // 16 MB
const VERTEX_BUFFER_ALIGNMENT: u64 = 16;
const INDEX_BUFFER_ALIGNMENT: u64 = 4;
const STORAGE_BUFFER_ALIGNMENT: u64 = 16;
const UNIFORM_BUFFER_ALIGNMENT: u64 = 256;

pub(crate) struct ResourceStore {
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

impl ResourceStore {
    pub fn new(dvc_ctx: &DeviceContext, swc_ctx: &SwapchainContext) -> Result<Self> {
        log::info!("Creating ResourceStore");

        let vertex_megabuffer = dvc_ctx.create_megabuffer(
            VERTEX_BUFFER_SIZE,
            VERTEX_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let index_megabuffer = dvc_ctx.create_megabuffer(
            INDEX_BUFFER_SIZE,
            INDEX_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let per_frame_megabuffer = dvc_ctx.create_megabuffer(
            PER_FRAME_BUFFER_SIZE,
            UNIFORM_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let per_material_megabuffer = dvc_ctx.create_megabuffer(
            PER_MATERIAL_BUFFER_SIZE,
            STORAGE_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let per_object_megabuffer = dvc_ctx.create_megabuffer(
            PER_OBJECT_BUFFER_SIZE,
            STORAGE_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let bindless_material_factory = dvc_ctx.create_bindless_material_factory()?;

        let fullscreen_quad = FullscreenQuad::new(&vertex_megabuffer, &index_megabuffer, swc_ctx)?;

        let mut samplers = Vec::new();
        samplers.push(
            dvc_ctx.create_vk_sampler(
                &vk::SamplerCreateInfo::default()
                    .mag_filter(vk::Filter::NEAREST)
                    .min_filter(vk::Filter::NEAREST)
                    .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                    .address_mode_u(vk::SamplerAddressMode::REPEAT)
                    .address_mode_v(vk::SamplerAddressMode::REPEAT)
                    .address_mode_w(vk::SamplerAddressMode::REPEAT),
            )?,
        );

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

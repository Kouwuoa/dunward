//! Long-lived GPU resource store.
//!
//! Exposes [`ResourceStore`], owning pooled megabuffers (vertex, index, per-frame,
//! per-material, per-object), textures, and managed samplers.

use ash::vk;
use color_eyre::Result;

use super::factory::ResourceFactory;
use super::megabuffer::Megabuffer;
use super::texture::{ColorTexture, StorageTexture};
use crate::material::MaterialFactory;

const VERTEX_BUFFER_SIZE: u64 = 1024 * 1024 * 256; // 256 MB
const INDEX_BUFFER_SIZE: u64 = 1024 * 1024 * 64; // 64 MB
const PER_FRAME_BUFFER_SIZE: u64 = 16 * 1024 * 1024; // 16 MB
const PER_MATERIAL_BUFFER_SIZE: u64 = 16 * 1024 * 1024; // 16 MB
const PER_OBJECT_BUFFER_SIZE: u64 = 16 * 1024 * 1024; // 16 MB
const VERTEX_BUFFER_ALIGNMENT: u64 = 16;
const INDEX_BUFFER_ALIGNMENT: u64 = 4;
const STORAGE_BUFFER_ALIGNMENT: u64 = 16;
const UNIFORM_BUFFER_ALIGNMENT: u64 = 256;

/// Owns resource lifetimes/data
pub(crate) struct ResourceStore {
    /// Offscreen compute render targets and G-buffers
    pub storage_textures: Vec<StorageTexture>,
    /// Albedo, normal, and material texture maps
    pub sampled_textures: Vec<ColorTexture>,
    /// Nearest, Linear, Repeat, Clamp texture sampling configs
    pub samplers: Vec<vk::Sampler>,

    pub vertex_megabuffer: Megabuffer,
    pub index_megabuffer: Megabuffer,
    pub per_material_megabuffer: Megabuffer,
    pub per_frame_megabuffer: Megabuffer,
    pub per_object_megabuffer: Megabuffer,

    /// Compute pipeline factory that creates material instances for compute passes
    pub compute_material_factory: MaterialFactory,
    /// Rasterization pipeline factory that creates material instances for rasterizing 3D geometry
    pub graphics_material_factory: MaterialFactory,
}

impl ResourceStore {
    pub fn new(factory: &ResourceFactory) -> Result<Self> {
        log::info!("Creating ResourceStore");

        let vertex_megabuffer = factory.create_megabuffer(
            VERTEX_BUFFER_SIZE,
            VERTEX_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let index_megabuffer = factory.create_megabuffer(
            INDEX_BUFFER_SIZE,
            INDEX_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let per_frame_megabuffer = factory.create_megabuffer(
            PER_FRAME_BUFFER_SIZE,
            UNIFORM_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let per_material_megabuffer = factory.create_megabuffer(
            PER_MATERIAL_BUFFER_SIZE,
            STORAGE_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let per_object_megabuffer = factory.create_megabuffer(
            PER_OBJECT_BUFFER_SIZE,
            STORAGE_BUFFER_ALIGNMENT,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_DST,
        )?;

        let compute_material_factory = factory.create_compute_material_factory()?;

        Ok(Self {
            storage_textures: Vec::new(),
            sampled_textures: Vec::new(),
            samplers: Vec::new(),

            vertex_megabuffer,
            index_megabuffer,
            per_frame_megabuffer,
            per_material_megabuffer,
            per_object_megabuffer,

            compute_material_factory,
        })
    }

    pub fn add_sampler(&mut self, sampler: vk::Sampler) {
        self.samplers.push(sampler);
    }
}

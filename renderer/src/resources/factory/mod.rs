//! GPU Resource Factory for constructing buffers, textures, and material factories.
//!
//! Exposes [`ResourceFactory`], providing creation methods for [`Megabuffer`],
//! [`Texture`], [`ColorTexture`], [`DepthTexture`], and [`StorageTexture`].

use ash::vk;
use color_eyre::Result;
use shaderpack::ShaderId;
use std::sync::{Arc, Mutex};

use super::ResourceType;
use super::texture::{ColorTexture, DepthTexture, StorageTexture, Texture};
use crate::commands::transfer::TransferCommandRecorder;
use crate::gpu::Gpu;
use crate::material::shader::{ComputeShader, GraphicsShader};
use crate::material::shader_data::PerDrawData;
use crate::material::{
    ComputeMaterialFactoryBuilder, GraphicsMaterialFactoryBuilder, MaterialFactory,
};
use crate::resources::deletion::sender::DeletionSender;
use crate::resources::descriptor::{DescriptorAllocator, DescriptorSetLayoutBuilder};
use crate::resources::megabuffer::Megabuffer;

mod compute;
mod graphics;

/// `ResourceFactory` is responsible only for construction (`create_*` APIs)
/// such as buffers, textures, and materials as needed.
/// It does not own long-lived storage, caching, or lookup tables.
pub(crate) struct ResourceFactory {
    transfer_command_recorder: Arc<Mutex<TransferCommandRecorder>>,
    descriptor_allocator: Arc<Mutex<DescriptorAllocator>>,
    gpu: Arc<Gpu>,
    deletion_sender: DeletionSender,
}

impl ResourceFactory {
    pub fn new(gpu: Arc<Gpu>, deletion_sender: DeletionSender) -> Result<Self> {
        let transfer_command_recorder = Arc::new(Mutex::new(TransferCommandRecorder::new(&gpu)?));
        let descriptor_allocator = Arc::new(Mutex::new(DescriptorAllocator::new(
            gpu.raw_logical(),
            1000,
        )?));
        Ok(Self {
            transfer_command_recorder,
            descriptor_allocator,
            gpu,
            deletion_sender,
        })
    }

    pub fn create_color_texture(
        &mut self,
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
            self.gpu.clone(),
            &mut *self.transfer_command_recorder.lock().unwrap(),
            self.deletion_sender.clone(),
        )
    }

    pub fn create_depth_texture(&self, width: u32, height: u32) -> Result<DepthTexture> {
        Texture::new_depth_texture(
            width,
            height,
            self.gpu.clone(),
            self.deletion_sender.clone(),
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
            self.transfer_command_recorder.clone(),
            self.deletion_sender.clone(),
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
            self.gpu.clone(),
            self.deletion_sender.clone(),
        )
    }
}

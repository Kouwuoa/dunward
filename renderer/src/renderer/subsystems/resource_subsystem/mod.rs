use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::subsystems::resource_subsystem::resource_binder::ResourceBinder;
use crate::renderer::subsystems::resource_subsystem::resource_store::ResourceStore;
use color_eyre::eyre::Result;
use std::sync::{Arc, Mutex};

pub(crate) mod resource_binder;
pub(crate) mod resource_store;

pub(crate) struct ResourceSubsystem {
    resource_store: ResourceStore,
    resource_binder: ResourceBinder,
    memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
}

impl ResourceSubsystem {
    pub fn new(dvc: &DeviceContext) -> Result<Self> {
        let memory_allocator = unsafe {
            vk_mem::Allocator::new(vk_mem::AllocatorCreateInfo::new(
                &dvc.instance_handle(),
                &dvc.logical_device_handle(),
                dvc.physical_device_handle(),
            ))?
        };

        Ok(Self {
            memory_allocator: Arc::new(Mutex::new(memory_allocator)),
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
            self.device.logical.clone(),
            &self.transfer_command_recorder,
        )
    }

    pub fn create_depth_texture(&self, width: u32, height: u32) -> Result<DepthTexture> {
        Texture::new_depth_texture(
            width,
            height,
            self.memory_allocator.clone(),
            self.device.logical.clone(),
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
            self.memory_allocator.clone(),
            self.device.logical.clone(),
            self.transfer_command_recorder.clone(),
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
            self.device.logical.clone(),
        )
    }

    pub fn create_bindless_material_factory(&self) -> Result<MaterialFactory> {
        let bindless_descriptor_set_layout = self.create_bindless_descriptor_set_layout()?;
        let bindless_pipeline_layout =
            self.create_bindless_pipeline_layout(bindless_descriptor_set_layout)?;
        let default_shader = ComputeShader::new("sky", self.device.logical.clone())?;
        ComputeMaterialFactoryBuilder::new(
            self.device.logical.clone(),
            self.descriptor_allocator.clone(),
        )
        .with_shader(default_shader)
        .with_pipeline_layout(bindless_pipeline_layout)
        .with_descriptor_set_layout(bindless_descriptor_set_layout)
        .build()
    }
}

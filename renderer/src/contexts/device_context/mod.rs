pub(crate) mod commands;
pub(crate) mod queue;

mod descriptors;
mod device;
mod instance;
mod surface;

use crate::contexts::device_context::queue::Queue;
use crate::contexts::{
    device_context::commands::{
        CommandRecorderAllocator, CommandRecorderAllocatorExt, TransferCommandRecorder,
    },
    swapchain_context::SwapchainContext,
};
use crate::resource_store::material::{GraphicsMaterialFactoryBuilder, MaterialFactory};
use crate::resource_store::megabuffer::{Megabuffer, MegabufferExt};
use crate::resource_store::resource_type::ResourceType;
use crate::resource_store::shader::GraphicsShader;
use crate::resource_store::shader_data::PerDrawData;
use crate::resource_store::texture::{ColorTexture, DepthTexture, StorageTexture, Texture};
use ash::vk;
use color_eyre::Result;
use descriptors::descriptor_set_layout_builder::DescriptorSetLayoutBuilder;
use gpu_descriptor::DescriptorAllocator;
use gpu_descriptor_ash::AshDescriptorDevice;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Main abstraction around the graphics API context for rendering.
pub(crate) struct DeviceContext {
    instance: instance::Instance,
    device: device::Device,
    surface: surface::Surface,

    pub(crate) memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
    pub(crate) descriptor_allocator:
        Arc<Mutex<DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet>>>,
    pub(crate) command_recorder_allocator: CommandRecorderAllocator,
    pub(crate) transfer_command_recorder: Arc<TransferCommandRecorder>,
}

impl DeviceContext {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        log::info!("Creating DeviceContext");

        let instance = instance::Instance::new(Some(window))?;
        let mut surface = instance.create_surface(window)?;
        let device = instance.create_device(&surface)?;
        let _ = surface.generate_surface_formats(&device)?;
        let _ = surface.generate_surface_present_modes(&device)?;

        let memory_allocator = unsafe {
            vk_mem::Allocator::new(vk_mem::AllocatorCreateInfo::new(
                instance.inner(),
                &device.logical,
                device.physical,
            ))?
        };

        let descriptor_allocator: DescriptorAllocator<vk::DescriptorPool, vk::DescriptorSet> =
            DescriptorAllocator::new(
                ResourceType::max_update_after_bind_descriptors_in_all_pools(),
            );

        let command_recorder_allocator = CommandRecorderAllocator::new(device.logical.clone())?;
        let transfer_command_recorder = Arc::new(TransferCommandRecorder::new(
            device.get_transfer_queue().clone(),
            device.logical.clone(),
        )?);

        Ok(Self {
            instance,
            device,
            surface,
            memory_allocator: Arc::new(Mutex::new(memory_allocator)),
            descriptor_allocator: Arc::new(Mutex::new(descriptor_allocator)),
            command_recorder_allocator,
            transfer_command_recorder,
        })
    }

    pub fn raw_device_handle(&self) -> vk::Device {
        self.device.logical.handle()
    }

    pub fn ash_device_handle(&self) -> Arc<ash::Device> {
        self.device.logical.clone()
    }

    pub fn get_graphics_queue(&self) -> Arc<Queue> {
        self.device.get_graphics_queue()
    }

    pub fn get_present_queue(&self) -> Arc<Queue> {
        self.device.get_present_queue()
    }

    pub fn wait_and_reset_fence(&self, fence: vk::Fence, timeout: Duration) -> Result<()> {
        unsafe {
            let fences = [fence];
            self.device
                .logical
                .wait_for_fences(&fences, true, timeout.as_nanos() as u64)?;
            self.device.logical.reset_fences(&fences)?;
        }
        Ok(())
    }

    pub fn wait_device_idle(&self) -> Result<()> {
        Ok(unsafe { self.device.logical.device_wait_idle()? })
    }

    pub fn create_swapchain_context(
        &self,
        window: &winit::window::Window,
    ) -> Result<SwapchainContext> {
        SwapchainContext::new(window, self)
    }

    pub fn create_color_texture(
        &self,
        width: u32,
        height: u32,
        data: Option<&[u8]>,
        use_dedicated_memory: bool,
    ) -> Result<ColorTexture> {
        Texture::new_color_texture_from_bytes(
            width,
            height,
            data,
            use_dedicated_memory,
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
        let default_shader = GraphicsShader::new("default", self.device.logical.clone())?;
        GraphicsMaterialFactoryBuilder::new(
            self.device.logical.clone(),
            self.descriptor_allocator.clone(),
        )
        .with_shader(default_shader)
        .with_pipeline_layout(bindless_pipeline_layout)
        .with_descriptor_set_layout(bindless_descriptor_set_layout)
        .with_color_attachment_format(vk::Format::R8G8B8A8_SRGB)
        .with_depth_attachment_format(vk::Format::D32_SFLOAT)
        .build()
    }

    pub fn create_bindless_descriptor_set_layout(&self) -> Result<vk::DescriptorSetLayout> {
        DescriptorSetLayoutBuilder::new()
            .add_binding(
                // Per-frame
                0,
                ResourceType::UniformBuffer.descriptor_type(),
                ResourceType::UniformBuffer.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                ResourceType::UniformBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Per-material
                1,
                ResourceType::StorageBuffer.descriptor_type(),
                ResourceType::StorageBuffer.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                ResourceType::StorageBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Per-object
                2,
                ResourceType::StorageBuffer.descriptor_type(),
                ResourceType::StorageBuffer.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                ResourceType::StorageBuffer.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Samplers
                3,
                ResourceType::Sampler.descriptor_type(),
                ResourceType::Sampler.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                ResourceType::Sampler.descriptor_binding_flags(),
                None,
            )
            .add_binding(
                // Textures
                4,
                ResourceType::SampledImage.descriptor_type(),
                ResourceType::SampledImage.descriptor_count(),
                vk::ShaderStageFlags::ALL,
                ResourceType::SampledImage.descriptor_binding_flags(),
                None,
            )
            .build(
                vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL,
                &self.device.logical,
            )
    }

    pub fn create_bindless_pipeline_layout(
        &self,
        bindless_descriptor_set_layout: vk::DescriptorSetLayout,
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

        let pipeline_layout = unsafe {
            self.device
                .logical
                .create_pipeline_layout(&pipeline_layout_create_info, None)?
        };

        Ok(pipeline_layout)
    }

    pub fn submit(
        &self,
        cmd: vk::CommandBuffer,
        queue: Arc<Queue>,
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
        fence: vk::Fence,
    ) -> Result<()> {
        self.device
            .submit(cmd, queue, wait_semaphores, signal_semaphores, fence)
    }
    pub fn create_vk_image_view(&self, info: &vk::ImageViewCreateInfo) -> Result<vk::ImageView> {
        Ok(unsafe { self.device.logical.create_image_view(info, None)? })
    }

    pub fn create_vk_sampler(&self, info: &vk::SamplerCreateInfo) -> Result<vk::Sampler> {
        Ok(unsafe { self.device.logical.create_sampler(info, None)? })
    }

    pub fn create_vk_semaphore(&self, info: &vk::SemaphoreCreateInfo) -> Result<vk::Semaphore> {
        Ok(unsafe { self.device.logical.create_semaphore(info, None)? })
    }

    pub fn create_vk_fence(&self, info: &vk::FenceCreateInfo) -> Result<vk::Fence> {
        Ok(unsafe { self.device.logical.create_fence(info, None)? })
    }

    pub fn create_vk_swapchain_loader(&self) -> ash::khr::swapchain::Device {
        ash::khr::swapchain::Device::new(self.instance.inner(), &self.device.logical)
    }
}
impl Drop for DeviceContext {
    fn drop(&mut self) {
        // Clean up descriptor sets
        let device = AshDescriptorDevice::wrap(&self.device.logical);
        unsafe {
            self.descriptor_allocator.lock().unwrap().cleanup(device);
        }
    }
}

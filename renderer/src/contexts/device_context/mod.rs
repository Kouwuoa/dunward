pub(crate) mod commands;
pub(crate) mod desc_set_layout_builder;
pub(crate) mod queue;

mod device;
mod instance;
mod surface;

use std::sync::{Arc, Mutex};
use crate::contexts::swapchain_context::SwapchainContext;
use crate::resources::texture::ColorTexture;
use ash::vk;
use color_eyre::Result;
use std::time::Duration;
use color_eyre::eyre::OptionExt;

/// Main abstraction around the graphics API context for rendering.
pub(crate) struct DeviceContext {
    instance: instance::Instance,
    device: device::Device,
    surface: surface::Surface,

    pub(crate) memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
}


impl DeviceContext {
    pub fn new(window: &winit::window::Window) -> Result<(Self, SwapchainContext)> {
        log::info!("Creating DeviceContext");

        let instance = instance::Instance::new(Some(window))?;
        let mut surface = instance.create_surface(window)?;
        let device = instance.create_device(&surface)?;
        let swapchain_context = instance.create_swapchain_context(&mut surface, window, &device)?;

        let memory_allocator = unsafe {
            vk_mem::Allocator::new(vk_mem::AllocatorCreateInfo::new(
                instance.inner(),
                &device.logical,
                device.physical,
            ))?
        };

        Ok((
            Self {
                instance,
                device,
                surface,
                memory_allocator: Arc::new(Mutex::new(memory_allocator)),
            },
            swapchain_context,
        ))
    }

    pub fn raw_device_handle(&self) -> vk::Device {
        self.device.logical.handle()
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

    pub fn create_color_texture(
        &self,
        width: u32,
        height: u32,
        data: Option<&[u8]>,
        use_dedicated_memory: bool,
    ) -> Result<ColorTexture> {
        self.device
            .create_color_texture(width, height, data, use_dedicated_memory)
    }

    pub fn create_image_view(&self, info: &vk::ImageViewCreateInfo) -> Result<vk::ImageView> {
        Ok(unsafe { self.device.logical.create_image_view(&info, None)? })
    }
}

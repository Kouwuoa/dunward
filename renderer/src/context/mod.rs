pub(crate) mod commands;
pub(crate) mod desc_set_layout_builder;
pub(crate) mod device;
pub(crate) mod instance;
pub(crate) mod queue;
pub(crate) mod target;

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use std::time::Duration;

/// Main abstraction around the graphics API context for rendering.
pub(crate) struct RenderContext {
    pub instance: instance::RenderInstance,
    pub device: device::RenderDevice,
    pub target: target::RenderTarget,
}

impl RenderContext {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
        log::info!("Creating RenderContext");

        let instance = instance::RenderInstance::new(Some(window))?;
        let surface = instance.create_surface(window)?;
        let device = instance.create_device(&surface)?;
        let target = instance.create_target(surface, window, &device)?;

        Ok(Self {
            instance,
            device,
            target,
        })
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

    pub fn acquire_next_swapchain_image(
        &self,
        semaphore: vk::Semaphore,
        timeout: Duration,
    ) -> Result<(SwapchainImage, SwapchainImageIndex, SwapchainImageExtent)> {
        let target = self
            .target
            .as_ref()
            .ok_or_eyre("Render target was not set")?;
        let (image_index, suboptimal) = unsafe {
            target.swapchain.swapchain_loader.acquire_next_image(
                target.swapchain.swapchain,
                timeout.as_nanos() as u64,
                semaphore,
                vk::Fence::null(),
            )?
        };
        if suboptimal {
            log::warn!("Acquired swapchain image is suboptimal. A resize may be necessary.");
        }

        let image = target
            .swapchain
            .swapchain_images
            .get(image_index as usize)
            .ok_or_eyre(eyre!(
                "Failed to get swapchain image at index {}",
                image_index
            ))?;

        let image_extent = target.swapchain.swapchain_image_extent;

        Ok((*image, image_index, image_extent))
    }
}

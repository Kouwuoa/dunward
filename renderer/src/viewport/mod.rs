mod surface;
mod swapchain;

use ash::vk;
use color_eyre::eyre::{OptionExt, eyre};
use std::time::Duration;
pub(crate) use surface::RenderSurface;

use crate::context::device::RenderDevice;
use crate::context::instance::RenderInstance;
use crate::viewport::swapchain::{SwapchainImage, SwapchainImageExtent, SwapchainImageIndex};
use color_eyre::Result;
use swapchain::RenderSwapchain;
use winit::window::Window;

pub(crate) struct PresentImage {
    pub image: SwapchainImage,
    pub index: SwapchainImageIndex,
    pub extent: SwapchainImageExtent,
    pub suboptimal: bool,
}

/// Presentation target of the renderer, encapsulating the surface and swapchain
pub(crate) struct RenderViewport {
    surface: RenderSurface,
    swapchain: RenderSwapchain,
}

impl RenderViewport {
    pub fn new(
        mut surface: RenderSurface,
        win: &Window,
        ins: &RenderInstance,
        dev: &RenderDevice,
    ) -> Result<Self> {
        log::info!("Creating RenderViewport");

        let _ = surface.generate_surface_formats(dev)?;
        let _ = surface.generate_surface_present_modes(dev)?;

        let swapchain = RenderSwapchain::new(&surface, &win.inner_size(), ins, dev)?;

        Ok(Self { surface, swapchain })
    }

    pub fn acquire_next_present_image(
        &self,
        image_acquired_sem: vk::Semaphore,
        timeout: Duration,
    ) -> Result<PresentImage> {
        let (image_index, suboptimal) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                timeout.as_nanos() as u64,
                image_acquired_sem,
                vk::Fence::null(),
            )?
        };
        if suboptimal {
            log::warn!("Acquired swapchain image is suboptimal. A resize may be necessary.");
        }

        let image = self
            .swapchain
            .swapchain_images
            .get(image_index as usize)
            .ok_or_eyre(eyre!(
                "Failed to get swapchain image at index {}",
                image_index
            ))?;

        let image_extent = self.swapchain.swapchain_image_extent;

        Ok(PresentImage {
            image: *image,
            index: image_index,
            extent: image_extent,
            suboptimal,
        })
    }

    pub fn present(&self, image: PresentImage) -> Result<bool> {
        let present_info = vk::PresentInfoKHR {
            p_swapchains: &self.swapchain.swapchain,
            swapchain_count: 1,
            p_wait_semaphores: &image.image.render_complete_semaphore, // Wait until rendering is done before presenting
            wait_semaphore_count: 1,
            p_image_indices: &image.index,
            ..Default::default()
        };

        let present_queue = self
            .swapchain
            .swapchain_loader
            .get_device()
            .graphics_queue
            .as_ref();
        assert!(present_queue.family.supports_present()); // Ensure the queue supports presentation

        let result = unsafe {
            self.swapchain
                .swapchain_loader
                .queue_present(present_queue.handle, &present_info)
        };

        match result {
            Ok(is_suboptimal) => Ok(is_suboptimal || image.suboptimal),
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => Ok(true), // Swapchain is out of date and needs to be recreated
            Err(e) => Err(eyre!("Failed to present swapchain image: {:?}", e)),
        }
    }

    pub fn resize(
        &mut self,
        size: winit::dpi::PhysicalSize<u32>,
        ins: &RenderInstance,
        dev: &RenderDevice,
    ) -> Result<()> {
        unsafe {
            dev.logical.device_wait_idle()?;
        }

        self.swapchain = RenderSwapchain::new(&self.surface, &size, ins, dev)?;

        Ok(())
    }

    pub fn get_size(&self) -> winit::dpi::PhysicalSize<u32> {
        winit::dpi::PhysicalSize::new(
            self.swapchain.swapchain_image_extent.width,
            self.swapchain.swapchain_image_extent.height,
        )
    }
}

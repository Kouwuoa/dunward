mod surface;
mod swapchain;

pub(crate) use surface::RenderSurface;

use crate::context::device::RenderDevice;
use crate::context::instance::RenderInstance;
use crate::context::queue::Queue;
use crate::viewport::swapchain::{SwapchainImage, SwapchainImageExtent, SwapchainImageIndex};
use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use std::sync::Arc;
use std::time::Duration;
use swapchain::RenderSwapchain;
use winit::window::Window;

pub(crate) struct PresentImage {
    pub image: SwapchainImage,
    pub index: SwapchainImageIndex,
    pub extent: SwapchainImageExtent,
    pub suboptimal: bool,
}

pub(crate) enum PresentResult {
    Success,
    ResizeRequested,
}

/// Presentation target of the renderer, encapsulating the surface and swapchain
pub(crate) struct RenderViewport {
    pub surface: RenderSurface,
    pub swapchain: RenderSwapchain,
    pub present_queue: Arc<Queue>,
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

        Ok(Self {
            surface,
            swapchain,
            present_queue: dev.get_present_queue(),
        })
    }

    pub fn acquire_next_present_image(
        &self,
        signal_image_acquired_sem: vk::Semaphore,
        timeout: Duration,
    ) -> Result<PresentImage> {
        let (image_index, suboptimal) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                timeout.as_nanos() as u64,
                signal_image_acquired_sem,
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

    pub fn present(
        &self,
        image: PresentImage,
        wait_render_finished_sem: vk::Semaphore,
    ) -> Result<PresentResult> {
        let swapchain_image_index = image.index;
        let present_info = vk::PresentInfoKHR {
            p_swapchains: &self.swapchain.swapchain,
            swapchain_count: 1,
            p_wait_semaphores: &wait_render_finished_sem, // Wait until rendering is done before presenting
            wait_semaphore_count: 1,
            p_image_indices: &swapchain_image_index,
            ..Default::default()
        };

        let present_queue = &self.present_queue;
        assert!(present_queue.family.supports_present()); // Ensure the queue supports presentation

        let present_result = unsafe {
            self.swapchain
                .swapchain_loader
                .queue_present(present_queue.handle, &present_info)
        };
        match present_result {
            Ok(true) => Ok(PresentResult::ResizeRequested),
            Ok(false) => Ok(PresentResult::Success),
            Err(err_code) => Err(eyre!(
                "Failed to present frame. VkResult error code: {}",
                err_code
            )),
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

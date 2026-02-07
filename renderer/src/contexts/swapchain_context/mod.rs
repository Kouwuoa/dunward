mod swapchain;

pub(crate) use crate::contexts::device_context::surface::RenderSurface;

use crate::resources::texture::{ColorTexture, Texture};
use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{eyre, OptionExt};
use std::sync::Arc;
use std::time::Duration;
use swapchain::Swapchain;
use winit::window::Window;
use crate::contexts::device_context::device::Device;
use crate::contexts::device_context::instance::Instance;
use crate::contexts::device_context::queue::Queue;

pub(crate) struct PresentTextureBundle {
    pub texture: ColorTexture,
    pub index: u32,
    pub suboptimal: bool,
}

pub(crate) enum PresentResult {
    Success,
    ResizeRequested,
}

/// Presentation target of the renderer, encapsulating the surface and swapchain
pub(crate) struct SwapchainContext {
    pub swapchain: Swapchain,
    pub present_queue: Arc<Queue>,
}

impl SwapchainContext {
    pub fn new(
        surface: &mut RenderSurface,
        win: &Window,
        ins: &Instance,
        dev: &Device,
    ) -> Result<Self> {
        log::info!("Creating RenderViewport");

        let _ = surface.generate_surface_formats(dev)?;
        let _ = surface.generate_surface_present_modes(dev)?;

        let swapchain = Swapchain::new(&surface, &win.inner_size(), ins, dev)?;

        Ok(Self {
            swapchain,
            present_queue: dev.get_present_queue(),
        })
    }

    pub fn acquire_next_present_texture(
        &self,
        signal_image_acquired_sem: vk::Semaphore,
        timeout: Duration,
        dev: &Device,
    ) -> Result<PresentTextureBundle> {
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
        let view = self
            .swapchain
            .swapchain_image_views
            .get(image_index as usize)
            .ok_or_eyre(eyre!(
                "Failed to get swapchain image view at index {}",
                image_index
            ))?;
        let format = &self.swapchain.swapchain_image_format;
        let extent = &self.swapchain.swapchain_image_extent;
        let texture = Texture::new_color_texture_from_vkimage(
            image,
            view,
            format,
            extent,
            false,
            dev.memory_allocator.clone(),
            dev.logical.clone(),
        );

        Ok(PresentTextureBundle {
            texture,
            index: image_index,
            suboptimal,
        })
    }

    pub fn present(
        &self,
        texture: PresentTextureBundle,
        wait_render_finished_sem: vk::Semaphore,
    ) -> Result<PresentResult> {
        let swapchain_image_index = texture.index;
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
        ins: &Instance,
        dev: &Device,
        sfc: &mut RenderSurface,
    ) -> Result<()> {
        unsafe {
            dev.logical.device_wait_idle()?;
        }

        self.swapchain = Swapchain::new(sfc, &size, ins, dev)?;

        Ok(())
    }

    pub fn get_size(&self) -> winit::dpi::PhysicalSize<u32> {
        winit::dpi::PhysicalSize::new(
            self.swapchain.swapchain_image_extent.width,
            self.swapchain.swapchain_image_extent.height,
        )
    }
}

//! Window Display Presentation, Backbuffers, and Frame Presentation.
//!
//! Exposes [`Display`], which manages the acquisition of next presentation targets,
//! queuing backbuffer images for presentation on the presentation queue, and handling window resizes.

pub(crate) mod swapchain;

pub(crate) use swapchain::Swapchain;

use crate::{
    gpu::{Gpu, queue::Queue, surface::Surface},
    resources::texture::ColorTexture,
};

use crate::gpu::semaphore::BinarySemaphore;
use crate::resources::texture::Texture;

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use winit::window::Window;

pub(crate) struct PresentTextureBundle {
    pub texture: ColorTexture,
    pub index: u32,
    pub suboptimal: bool,
}

#[derive(Debug, Error)]
pub(crate) enum DisplayPresentError {
    #[error("Display surface is suboptimal and needs to be resized")]
    DisplaySuboptimal,

    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

/// Presentation target of the renderer, encapsulating the display surface and swapchain
pub(crate) struct Display {
    surface: Surface,
    swapchain: Swapchain,
    present_queue: Arc<Queue>,
}

impl Display {
    pub fn new(window: &Window, gpu: Arc<Gpu>, surface: Surface) -> Result<Self> {
        let swapchain = Swapchain::new(&window.inner_size(), &gpu, &surface, None)?;

        Ok(Self {
            surface,
            swapchain,
            present_queue: gpu.get_present_queue(),
        })
    }

    pub fn acquire_next_present_texture(
        &self,
        signal_image_acquired_sem: &BinarySemaphore,
        timeout: Duration,
        gpu: Arc<Gpu>,
    ) -> Result<PresentTextureBundle> {
        let (image_index, suboptimal) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                timeout.as_nanos() as u64,
                signal_image_acquired_sem.raw(),
                vk::Fence::null(),
            )?
        };

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
            self.present_queue.clone(),
            gpu,
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
        wait_render_finished_sem: &BinarySemaphore,
    ) -> core::result::Result<(), DisplayPresentError> {
        let swapchain_image_index = texture.index;
        let present_info = vk::PresentInfoKHR {
            p_swapchains: &self.swapchain.swapchain,
            swapchain_count: 1,
            p_wait_semaphores: &wait_render_finished_sem.raw(), // Wait until rendering is done before presenting
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
            Ok(false) => Ok(()),
            Ok(true) => Err(DisplayPresentError::DisplaySuboptimal),
            Err(err) => Err(DisplayPresentError::Vulkan(err)),
        }
    }

    pub fn resize(&mut self, size: &winit::dpi::PhysicalSize<u32>, gpu: &Gpu) -> Result<()> {
        gpu.wait_idle()?;
        self.swapchain = Swapchain::new(size, &gpu, &self.surface, Some(&self.swapchain))?;
        Ok(())
    }

    pub fn get_size(&self) -> winit::dpi::PhysicalSize<u32> {
        winit::dpi::PhysicalSize::new(
            self.swapchain.swapchain_image_extent.width,
            self.swapchain.swapchain_image_extent.height,
        )
    }
}

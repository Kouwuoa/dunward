mod swapchain;

use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::contexts::device_context::queue::Queue;
use crate::renderer::contexts::device_context::semaphore::BinarySemaphore;
use crate::renderer::subsystems::resource_subsystem::ResourceSubsystem;
use crate::renderer::subsystems::resource_subsystem::resource_types::texture::{
    ColorTexture, Texture,
};
use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use swapchain::Swapchain;
use thiserror::Error;
use winit::window::Window;

pub(crate) struct PresentTextureBundle {
    pub texture: ColorTexture,
    pub index: u32,
    pub suboptimal: bool,
}

#[derive(Debug, Error)]
pub(crate) enum SwapchainPresentError {
    #[error("Swapchain is suboptimal and needs to be resized")]
    SwapchainSuboptimal,

    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

/// Presentation target of the renderer, encapsulating the surface and swapchain
pub(crate) struct SwapchainContext {
    pub(crate) swapchain: Swapchain,
    present_queue: Arc<Queue>,
    memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
    device: Arc<ash::Device>,
}

impl SwapchainContext {
    pub fn new(
        window: &Window,
        dvc_ctx: &DeviceContext,
        rsc_sys: &ResourceSubsystem,
    ) -> Result<Self> {
        log::info!("Creating SwapchainContext");

        let swapchain = Swapchain::new(&window.inner_size(), dvc_ctx, None)?;

        Ok(Self {
            swapchain,
            present_queue: dvc_ctx.get_present_queue(),
            memory_allocator: rsc_sys.get_memory_allocator(),
            device: dvc_ctx.logical_device_handle(),
        })
    }

    pub fn acquire_next_present_texture(
        &self,
        signal_image_acquired_sem: &BinarySemaphore,
        timeout: Duration,
    ) -> Result<PresentTextureBundle> {
        let (image_index, suboptimal) = unsafe {
            self.swapchain.swapchain_loader.acquire_next_image(
                self.swapchain.swapchain,
                timeout.as_nanos() as u64,
                signal_image_acquired_sem.raw(),
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
            self.memory_allocator.clone(),
            self.device.clone(),
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
    ) -> core::result::Result<(), SwapchainPresentError> {
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
            Ok(false) => Ok(()),
            Ok(true) => Err(SwapchainPresentError::SwapchainSuboptimal),
            Err(err) => Err(SwapchainPresentError::Vulkan(err)),
        }
    }

    pub fn resize(
        &mut self,
        size: &winit::dpi::PhysicalSize<u32>,
        dvc_ctx: &DeviceContext,
    ) -> Result<()> {
        dvc_ctx.wait_device_idle()?;

        self.swapchain = Swapchain::new(size, dvc_ctx, Some(&self.swapchain))?;

        Ok(())
    }

    pub fn get_size(&self) -> winit::dpi::PhysicalSize<u32> {
        winit::dpi::PhysicalSize::new(
            self.swapchain.swapchain_image_extent.width,
            self.swapchain.swapchain_image_extent.height,
        )
    }
}

use ash::vk;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use std::sync::Arc;
use winit::window::Window;
use super::instance::RenderInstance;
use super::device::RenderDevice;
use super::swapchain::Swapchain;

/// Presentation target of the renderer, encapsulating the window, surface, and swapchain
pub(crate) struct RenderTarget {
    pub surface: vk::SurfaceKHR,
    pub surface_loader: ash::khr::surface::Instance,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_present_mode: vk::PresentModeKHR,
    pub swapchain: Swapchain,
}

impl RenderTarget {
    pub fn new(
        window: &Window,
        surface: (vk::SurfaceKHR, ash::khr::surface::Instance),
        ins: &RenderInstance,
        dev: &RenderDevice,
    ) -> Result<Self> {
        let surface_loader = surface.1;
        let surface = surface.0;

        let surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(dev.physical, surface)?
        };

        let surface_present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(dev.physical, surface)?
        };

        let surface_format = surface_formats
            .iter()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_SRGB
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .ok_or_eyre("No suitable surface format found")?;

        let surface_present_mode = surface_present_modes
            .iter()
            .find(|mode| **mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(&vk::PresentModeKHR::FIFO);

        let swapchain = Swapchain::new(
            &surface,
            &surface_loader,
            surface_format,
            surface_present_mode,
            &window.inner_size(),
            ins,
            dev,
        )?;

        Ok(Self {
            surface,
            surface_loader,
            surface_format: *surface_format,
            surface_present_mode: *surface_present_mode,
            swapchain,
        })
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

        self.swapchain = Swapchain::new(
            &self.surface,
            &self.surface_loader,
            &self.surface_format,
            &self.surface_present_mode,
            &size,
            ins,
            dev,
        )?;

        Ok(())
    }

    pub fn get_size(&self) -> winit::dpi::PhysicalSize<u32> {
        winit::dpi::PhysicalSize::new(
            self.swapchain.swapchain_image_extent.width,
            self.swapchain.swapchain_image_extent.height,
        )
    }

}
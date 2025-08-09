use crate::renderer::contexts::device_ctx::swapchain::Swapchain;
use ash::vk;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use std::sync::Arc;
use winit::window::Window;
use crate::renderer::contexts::device_ctx::device::RenderDevice;
use crate::renderer::contexts::device_ctx::instance::RenderInstance;

/// Presentation target of the renderer, encapsulating the window, surface, and swapchain
pub(super) struct RenderTarget {
    pub window: Arc<Window>,

    pub surface: vk::SurfaceKHR,
    pub surface_loader: ash::khr::surface::Instance,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_present_mode: vk::PresentModeKHR,

    pub swapchain: Swapchain,
}

impl RenderTarget {
    pub fn new(
        window: Arc<Window>,
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
            &window,
            ins,
            dev,
        )?;

        Ok(Self {
            window,
            surface,
            surface_loader,
            surface_format: *surface_format,
            surface_present_mode: *surface_present_mode,
            swapchain,
        })
    }

    pub fn resize(
        &mut self,
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
            &self.window,
            ins,
            dev,
        )?;

        Ok(())
    }

    pub fn get_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.window.inner_size()
    }

}
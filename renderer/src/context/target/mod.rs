mod surface;
mod swapchain;

pub(crate) use surface::Surface;

use super::device::RenderDevice;
use super::instance::RenderInstance;
use color_eyre::Result;
use swapchain::Swapchain;
use winit::window::Window;

/// Presentation target of the renderer, encapsulating the window, surface, and swapchain
pub(crate) struct RenderTarget {
    surface: Surface,
    swapchain: Swapchain,
}

impl RenderTarget {
    pub fn new(mut surface: Surface, win: &Window, ins: &RenderInstance, dev: &RenderDevice) -> Result<Self> {
        let _ = surface.generate_surface_formats(dev)?;
        let _ = surface.generate_surface_present_modes(dev)?;

        let swapchain = Swapchain::new(
            &surface,
            &win.inner_size(),
            ins,
            dev,
        )?;

        Ok(Self {
            surface,
            swapchain,
        })
    }

    pub fn acquire_next_texture(&self) {

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

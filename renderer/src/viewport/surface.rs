use crate::context::device::RenderDevice;
use ash::vk;
use color_eyre::{Result, eyre::eyre};

pub(crate) struct RenderSurface {
    pub surface: vk::SurfaceKHR,
    pub surface_loader: ash::khr::surface::Instance,
    pub surface_formats: Vec<vk::SurfaceFormatKHR>,
    pub surface_present_modes: Vec<vk::PresentModeKHR>,
}

impl RenderSurface {
    pub(crate) fn generate_surface_present_modes(
        &mut self,
        dev: &RenderDevice,
    ) -> Result<&Vec<vk::PresentModeKHR>> {
        if !self.surface_present_modes.is_empty() {
            return Err(eyre!("Surface present modes have already been generated"));
        }

        self.surface_present_modes = unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(dev.physical, self.surface)?
        };

        Ok(&self.surface_present_modes)
    }

    pub(crate) fn generate_surface_formats(
        &mut self,
        dev: &RenderDevice,
    ) -> Result<&Vec<vk::SurfaceFormatKHR>> {
        if !self.surface_formats.is_empty() {
            return Err(eyre!("Surface formats have already been generated"));
        }

        self.surface_formats = unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(dev.physical, self.surface)?
        };

        Ok(&self.surface_formats)
    }
}

use ash::vk;
use color_eyre::{Result, eyre::eyre};
use color_eyre::eyre::OptionExt;
use crate::contexts::device_context::device::Device;
use crate::contexts::device_context::DeviceContext;

pub(crate) struct Surface {
    pub(super) surface: vk::SurfaceKHR,
    pub(super) surface_loader: ash::khr::surface::Instance,
    pub(super) surface_formats: Vec<vk::SurfaceFormatKHR>,
    pub(super) surface_present_modes: Vec<vk::PresentModeKHR>,
}

impl Surface {
    pub(crate) fn generate_surface_present_modes(
        &mut self,
        dev: &Device,
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
        dev: &Device,
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

/// Surface-related methods for DeviceContext
impl DeviceContext {
    pub(crate) fn raw_surface_handle(&self) -> vk::SurfaceKHR {
        self.surface.surface
    }

    pub(crate) fn get_physical_device_surface_capabilities(
        &self,
    ) -> Result<vk::SurfaceCapabilitiesKHR> {
        Ok(unsafe {
            self.surface
                .surface_loader
                .get_physical_device_surface_capabilities(
                    self.device.physical,
                    self.surface.surface,
                )?
        })
    }

    pub(crate) fn find_suitable_surface_format(&self) -> Result<vk::SurfaceFormatKHR> {
        self.surface
            .surface_formats
            .iter()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_SRGB
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .copied()
            .ok_or_eyre("No suitable surface format found")
    }

    pub(crate) fn find_suitable_surface_present_mode(&self) -> vk::PresentModeKHR {
        *self.surface
            .surface_present_modes
            .iter()
            .find(|mode| **mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(&vk::PresentModeKHR::FIFO)
    }
}


//! Window surface abstraction and presentation format queries.
//!
//! Manages [`ash::vk::SurfaceKHR`], surface capabilities, supported color formats,
//! and presentation modes (FIFO, Mailbox) for the window.

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};

use crate::core::DeviceContext;
use crate::core::device::Device;

pub(crate) struct Surface {
    pub surface: vk::SurfaceKHR,
    pub surface_loader: ash::khr::surface::Instance,
    pub surface_formats: Vec<vk::SurfaceFormatKHR>,
    pub surface_present_modes: Vec<vk::PresentModeKHR>,
}

impl Surface {
    pub fn generate_surface_present_modes(
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

    pub fn generate_surface_formats(
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
    pub fn raw_surface_handle(&self) -> vk::SurfaceKHR {
        self.surface.surface
    }

    pub fn get_physical_device_surface_capabilities(
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

    pub fn find_suitable_surface_format(&self) -> Result<vk::SurfaceFormatKHR> {
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

    pub fn find_suitable_surface_present_mode(&self) -> vk::PresentModeKHR {
        *self
            .surface
            .surface_present_modes
            .iter()
            .find(|mode| **mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(&vk::PresentModeKHR::FIFO)
    }
}

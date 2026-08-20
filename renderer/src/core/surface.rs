//! Window surface abstraction and presentation format queries.
//!
//! Manages [`ash::vk::SurfaceKHR`], surface capabilities, supported color formats,
//! and presentation modes (FIFO, Mailbox) for the window.

use ash::vk;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};

use crate::core::device::Device;

pub(crate) struct Surface {
    surface: vk::SurfaceKHR,
    surface_loader: ash::khr::surface::Instance,
    surface_formats: Vec<vk::SurfaceFormatKHR>,
    surface_present_modes: Vec<vk::PresentModeKHR>,
}

impl Surface {
    pub fn init_surface_present_modes(&mut self, device: &Device) -> Result<()> {
        if !self.surface_present_modes.is_empty() {
            return Err(eyre!("Surface present modes have already been generated"));
        }

        self.surface_present_modes = unsafe {
            self.surface_loader
                .get_physical_device_surface_present_modes(device.raw_physical(), self.surface)?
        };

        Ok(())
    }

    pub fn init_surface_formats(&mut self, device: &Device) -> Result<()> {
        if !self.surface_formats.is_empty() {
            return Err(eyre!("Surface formats have already been generated"));
        }

        self.surface_formats = unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(device.raw_physical(), self.surface)?
        };

        Ok(())
    }

    pub fn get_physical_device_surface_capabilities(
        &self,
        device: &Device,
    ) -> Result<vk::SurfaceCapabilitiesKHR> {
        Ok(unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(device.raw_physical(), self.surface)?
        })
    }

    pub fn find_suitable_surface_format(&self) -> Result<vk::SurfaceFormatKHR> {
        self.surface_formats
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
            .surface_present_modes
            .iter()
            .find(|mode| **mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(&vk::PresentModeKHR::FIFO)
    }

    pub fn raw(&self) -> vk::SurfaceKHR {
        self.surface
    }
}

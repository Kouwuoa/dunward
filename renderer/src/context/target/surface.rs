use crate::context::device::RenderDevice;
use ash::vk;
use color_eyre::Result;

pub(crate) struct Surface {
    pub surface: vk::SurfaceKHR,
    pub surface_loader: ash::khr::surface::Instance,
    pub surface_formats: Vec<vk::SurfaceFormatKHR>,
    pub surface_present_modes: Vec<vk::PresentModeKHR>,
}

impl Surface {
    pub(crate) fn generate_surface_present_modes(
        &mut self,
        dev: &RenderDevice,
    ) -> Result<&Vec<vk::PresentModeKHR>> {
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
        self.surface_formats = unsafe {
            self.surface_loader
                .get_physical_device_surface_formats(dev.physical, self.surface)?
        };
        Ok(&self.surface_formats)
    }
}

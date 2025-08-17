use ash::vk;
use color_eyre::Result;

pub(crate) mod commands;
pub(crate) mod desc_set_layout_builder;
pub(crate) mod device;
pub(crate) mod instance;
pub(crate) mod queue;
pub(crate) mod target;
pub(crate) mod swapchain;

/// Main abstraction around the graphics API context for rendering.
pub(crate) struct RenderContext {
    pub instance: instance::RenderInstance,
    pub device: device::RenderDevice,
    pub target: Option<target::RenderTarget>,
}

impl RenderContext {
    pub fn new(window: Option<&winit::window::Window>) -> Result<Self> {
        log::info!("Creating RenderContext");

        let instance = instance::RenderInstance::new(window)?;
        let surface = if let Some(window) = window {
            Some(instance.create_surface(window)?)
        } else {
            None
        };
        let device = instance.create_device(surface.as_ref())?;
        let target = if let (Some(window), Some(surface)) = (window, surface) {
            Some(instance.create_target(window, surface, &device)?)
        } else {
            None
        };
        Ok(Self {
            instance,
            device,
            target,
        })
    }

    pub fn wait_and_reset_fence(&self, fence: vk::Fence, timeout: u64) -> Result<()> {
        unsafe {
            let fences = [fence];
            self.device
                .logical
                .wait_for_fences(&fences, true, timeout)?;
            self.device.logical.reset_fences(&fences)?;
        }
        Ok(())
    }
}

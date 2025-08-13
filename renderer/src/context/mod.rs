use color_eyre::Result;
pub(super) mod commands;
mod desc_set_layout_builder;
mod device;
mod instance;
mod queue;
mod target;
mod swapchain;

/// Main abstraction around the graphics API context for rendering.
pub(super) struct RenderContext {
    pub instance: instance::RenderInstance,
    pub device: device::RenderDevice,
    pub target: Option<target::RenderTarget>,
}

impl RenderContext {
    pub fn new(window: Option<&winit::window::Window>) -> Result<Self> {
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
}

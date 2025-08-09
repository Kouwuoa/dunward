mod instance;
mod device;
mod target;

/// Main abstraction around the graphics API context for rendering.
pub(super) struct RenderContext {
    pub instance: instance::RenderInstance,
    pub device: device::RenderDevice,
    pub target: Option<target::RenderTarget>
}

impl RenderContext {
    pub fn new(window: Option<&winit::window::Window>) -> Self {
        let instance = instance::RenderInstance::new(window);
        Self { instance }
    }
}
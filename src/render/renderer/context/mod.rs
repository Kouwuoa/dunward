mod instance;

pub(super) struct RenderContext {
    pub instance: instance::RenderInstance,
}

impl RenderContext {
    pub fn new(window: Option<&winit::window::Window>) -> Self {
        let instance = instance::RenderInstance::new(window);
        Self { instance }
    }
}
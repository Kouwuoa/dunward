use super::camera::Camera;

mod context;

pub(super) struct Renderer {
    ctx: context::RenderContext,
}

impl Renderer {
    pub fn new(window: Option<&winit::window::Window>) -> Self {
        let ctx = context::RenderContext::new(window);
        Self { ctx }
    }

    pub fn render_frame(&mut self, cam: &Camera) {
        // Implement rendering logic here
        // This could involve drawing entities, handling camera views, etc.
    }
}

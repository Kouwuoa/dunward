mod context;
mod camera;
mod resources;

pub use camera::Camera;

pub struct Renderer {
    ctx: context::RenderContext,
}

impl Renderer {
    pub fn new(window: Option<&winit::window::Window>) -> Self {
        let _ = color_eyre::install();
        let _ = env_logger::try_init();


        let ctx = context::RenderContext::new(window);
        Self { ctx }
    }

    pub fn render_frame(&mut self, cam: &Camera) {
        // Implement rendering logic here
        // This could involve drawing entities, handling camera views, etc.
    }
}

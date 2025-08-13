use color_eyre::Result;

mod camera;
mod context;
mod resource_storage;
mod resource_type;
mod resources;
mod shader_data;

pub use camera::Camera;

pub struct Renderer {
    ctx: context::RenderContext,
}

impl Renderer {
    pub fn new(window: Option<&winit::window::Window>) -> Result<Self> {
        let _ = color_eyre::install();
        let _ = env_logger::try_init();

        let ctx = context::RenderContext::new(window)?;
        Ok(Self { ctx })
    }

    pub fn render_frame(&mut self, cam: &Camera) {
        // Implement rendering logic here
        // This could involve drawing entities, handling camera views, etc.
    }
}

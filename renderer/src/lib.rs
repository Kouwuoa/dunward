use color_eyre::Result;

mod camera;
mod context;
mod resources;
mod storage;
//mod frame;

pub use camera::Camera;

pub struct Renderer {
    ctx: context::RenderContext,
    sto: storage::RenderStorage,
    //frames: Vec<frame::RenderFrame>,
    resize_requested: bool,
}

impl Renderer {
    pub fn new(window: Option<&winit::window::Window>) -> Result<Self> {
        let _ = color_eyre::install();
        let _ = env_logger::try_init();

        let ctx = context::RenderContext::new(window)?;
        let sto = storage::RenderStorage::new(&ctx)?;

        Ok(Self {
            ctx,
            sto,
            //frames: Vec::new(),
            resize_requested: false,
        })
    }

    pub fn render_frame(&mut self, cam: &Camera) {
        // Implement rendering logic here
        // This could involve drawing entities, handling camera views, etc.
    }

    pub fn request_resize(&mut self) {
        self.resize_requested = true;
    }
}

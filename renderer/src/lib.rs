use color_eyre::Result;

mod camera;
mod context;
mod resources;
mod storage;
mod frame;

pub use camera::Camera;

pub struct Renderer {
    ctx: context::RenderContext,
    sto: storage::RenderStorage,
    frm: Vec<frame::RenderFrame>,

    current_frame_index: usize,
    resize_requested: bool,
}

impl Renderer {
    const FRAMES_IN_FLIGHT: usize = 1;

    pub fn new(window: Option<&winit::window::Window>) -> Result<Self> {
        let _ = color_eyre::install();
        let _ = env_logger::try_init();

        let mut ctx = context::RenderContext::new(window)?;
        let sto = storage::RenderStorage::new(&ctx)?;
        let frm = (0..Self::FRAMES_IN_FLIGHT)
            .map(|_| frame::RenderFrame::new(&mut ctx, &sto))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            ctx,
            sto,
            frm,
            current_frame_index: 0,
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

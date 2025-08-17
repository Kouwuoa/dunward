use color_eyre::Result;
use std::sync::{Arc, Mutex, RwLock};

mod camera;
mod context;
mod frame;
mod resources;
mod storage;

pub use camera::Camera;
use context::RenderContext;
use frame::RenderFrame;
use frame::packet::{FrameRenderMetadata, FrameRenderPacket, FrameRenderPayload};
use storage::RenderStorage;

pub struct Renderer {
    ctx: Arc<RwLock<RenderContext>>,
    sto: Arc<Mutex<RenderStorage>>,
    frm: Vec<Arc<RenderFrame>>,

    current_frame_index: usize,
    resize_requested: bool,
}

impl Renderer {
    const FRAMES_IN_FLIGHT: usize = 1;

    pub fn new(window: Option<&winit::window::Window>) -> Result<Self> {
        let _ = color_eyre::install();
        let _ = env_logger::try_init();

        let ctx = RenderContext::new(window)?;
        let sto = RenderStorage::new(&ctx)?;

        let ctx = Arc::new(RwLock::new(ctx));
        let frm = (0..Self::FRAMES_IN_FLIGHT)
            .map(|_| RenderFrame::new(ctx.clone(), &sto).map(Arc::new))
            .collect::<Result<Vec<_>>>()?;

        let sto = Arc::new(Mutex::new(sto));

        Ok(Self {
            ctx,
            sto,
            frm,
            current_frame_index: 0,
            resize_requested: false,
        })
    }

    pub fn render_frame(&mut self, cam: &Camera) -> Result<()> {
        self.current_frame_index = (self.current_frame_index + 1) % self.frm.len();
        let current_frame = self.frm[self.current_frame_index].clone();

        // Update the scene and prepare the frame packet
        let frame_pkt = self.update_scene(cam)?;

        // Record and submit the commands for the current frame
        current_frame.render(frame_pkt)?;

        // Present the frame
        //current_frame.present()?;

        Ok(())
    }

    pub fn request_resize(&mut self) {
        self.resize_requested = true;
    }

    fn update_scene<'a>(&mut self, cam: &'a Camera) -> Result<FrameRenderPacket<'a>> {
        let target_size = self
            .ctx
            .read()
            .map_err(|e| color_eyre::eyre::eyre!("Failed to read context: {}", e))?
            .target
            .as_ref()
            .ok_or_else(|| color_eyre::eyre::eyre!("Render target was not set"))?
            .get_size();
        let frame_metadata = FrameRenderMetadata {
            frame_index: self.current_frame_index,
            target_size,
            resize_requested: self.resize_requested,
        };
        Ok(FrameRenderPacket {
            payload: FrameRenderPayload { cam },
            metadata: frame_metadata,
        })
    }
}

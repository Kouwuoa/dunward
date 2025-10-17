mod camera;
mod context;
mod frame;
mod resources;
mod storage;
mod utils;
mod viewport;

pub use camera::Camera;

use crate::utils::GuardResultExt;
use crate::viewport::RenderViewport;
use color_eyre::eyre::OptionExt;
use color_eyre::Result;
use context::RenderContext;
use frame::packet::FrameRenderPacket;
use frame::packet::{FrameRenderMetadata, FrameRenderPayload};
use frame::RenderFrame;
use std::sync::{Arc, Mutex};
use storage::RenderStorage;

pub struct Renderer {
    ctx: Arc<Mutex<RenderContext>>,
    vpt: Arc<Mutex<RenderViewport>>,
    sto: Arc<Mutex<RenderStorage>>,
    frm: Vec<Arc<RenderFrame>>,

    current_frame_index: usize,
    resize_requested: bool,
}

impl Renderer {
    const FRAMES_IN_FLIGHT: usize = 1;

    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let _ = color_eyre::install();
        let _ = env_logger::try_init();

        let (ctx, vpt) = RenderContext::new(window)?;
        let sto = RenderStorage::new(&ctx, &vpt)?;

        let ctx = Arc::new(Mutex::new(ctx));
        let vpt = Arc::new(Mutex::new(vpt));
        let sto = Arc::new(Mutex::new(sto));
        let frm = (0..Self::FRAMES_IN_FLIGHT)
            .map(|_| RenderFrame::new(ctx.clone(), vpt.clone(), sto.clone()).map(Arc::new))
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            ctx,
            vpt,
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
        let render_pkt = self.update_scene(cam)?;

        // Record and submit the commands for the current frame
        let present_pkt = current_frame.render(render_pkt)?;

        // Present the frame
        match current_frame.present(present_pkt)? {
            viewport::PresentResult::ResizeRequested => {
                self.request_resize();
            }
            viewport::PresentResult::Success => {}
        }

        Ok(())
    }

    pub fn request_resize(&mut self) {
        self.resize_requested = true;
    }

    fn update_scene<'a>(&mut self, cam: &'a Camera) -> Result<FrameRenderPacket<'a>> {
        let target_size = self.vpt.lock().eyre()?.get_size();
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

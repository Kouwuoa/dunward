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
use color_eyre::Result;
use color_eyre::eyre::OptionExt;
use context::RenderContext;
use frame::RenderFrame;
use frame::packet::FrameRenderPacket;
use frame::packet::{FrameRenderMetadata, FrameRenderPayload};
use std::sync::{Arc, Mutex, MutexGuard};
use storage::RenderStorage;

/// We split up the Renderer by lifetimes:
/// * Tier 1 - Device lifetime (lives until app shutdown)
///   * Instance
///   * Physical Device
///   * Logical Device
///   * Queues
///   * Descriptor pool(s)
/// * Tier 2 - Swapchain lifetime (recreated on resize)
///   * Swapchain
///   * Swapchain images/views
///   * Framebuffers
///   * Render passes
///   * Viewport/extent
/// * Tier 3 - Per-frame lifetime (frames in flight)
///   * Command buffers
///   * Semaphores
///   * Fences
///   * Per-frame descriptor sets
pub struct Renderer {
    ctx: Arc<Mutex<RenderContext>>,
    vpt: Arc<Mutex<RenderViewport>>,
    sto: Arc<Mutex<RenderStorage>>,
    frm: Vec<RenderFrame>,

    frame_number: u64,
    resize_requested: bool,
}

impl Renderer {
    const FRAMES_IN_FLIGHT: u64 = 1;

    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let _ = color_eyre::install();
        let _ = env_logger::try_init();

        let (ctx, vpt) = RenderContext::new(window)?;
        let sto = RenderStorage::new(&ctx, &vpt)?;

        let ctx = Arc::new(Mutex::new(ctx));
        let vpt = Arc::new(Mutex::new(vpt));
        let sto = Arc::new(Mutex::new(sto));
        let frm = (0..Self::FRAMES_IN_FLIGHT)
            .map(|_| RenderFrame::new(ctx.clone(), vpt.clone(), sto.clone()))
            .collect::<Result<Vec<RenderFrame>>>()?;

        Ok(Self {
            ctx,
            vpt,
            sto,
            frm,
            frame_number: 0,
            resize_requested: false,
        })
    }

    pub fn render_frame(&mut self, cam: &Camera) -> Result<()> {
        // Update the scene and prepare the frame packet
        let render_pkt = self.update_scene(cam)?;

        // Record and submit the commands for the current frame
        let present_pkt = self.get_current_frame().render(render_pkt)?;

        // Present the frame
        match self.get_current_frame().present(present_pkt)? {
            viewport::PresentResult::ResizeRequested => {
                self.request_resize();
            }
            viewport::PresentResult::Success => {}
        }

        // Increment the frame counter
        self.frame_number += 1;

        Ok(())
    }

    pub fn request_resize(&mut self) {
        self.resize_requested = true;
    }

    fn update_scene<'a>(&mut self, cam: &'a Camera) -> Result<FrameRenderPacket<'a>> {
        let target_size = self.vpt.lock().eyre()?.get_size();
        let frame_metadata = FrameRenderMetadata {
            frame_index: self.get_current_frame_index(),
            target_size,
            resize_requested: self.resize_requested,
        };
        Ok(FrameRenderPacket {
            payload: FrameRenderPayload { cam },
            metadata: frame_metadata,
        })
    }

    fn get_current_frame(&mut self) -> &mut RenderFrame {
        let idx = self.get_current_frame_index();
        &mut self.frm[idx]
    }

    fn get_current_frame_index(&self) -> usize {
        (self.frame_number % Self::FRAMES_IN_FLIGHT) as usize
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.ctx.lock().unwrap().dev.logical.device_wait_idle().unwrap();
        }
    }
}

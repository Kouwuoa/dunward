mod camera;
mod contexts;
mod resource_store;
mod utils;

pub use camera::Camera;

use crate::contexts::frame_context::FrameContext;
use crate::contexts::frame_context::packet::{
    FrameRenderMetadata, FrameRenderPacket, FrameRenderPayload,
};
use crate::contexts::swapchain_context::{PresentResult, SwapchainContext};
use crate::resource_store::ResourceStore;
use crate::utils::GuardResultExt;
use color_eyre::Result;
use contexts::device_context::DeviceContext;
use std::sync::{Arc, Mutex};

pub struct Renderer {
    dvc_ctx: Arc<Mutex<DeviceContext>>,
    swc_ctx: Arc<Mutex<SwapchainContext>>,
    frm_ctxs: Vec<FrameContext>,
    rsc_sto: Arc<Mutex<ResourceStore>>,

    frame_number: u64,
    resize_requested: bool,
}

impl Renderer {
    const FRAMES_IN_FLIGHT: u64 = 1;

    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let _ = color_eyre::install();
        let _ = env_logger::try_init();

        let dvc_ctx = DeviceContext::new(window)?;
        let swc_ctx = dvc_ctx.create_swapchain_context(window)?;
        let rsc_sto = ResourceStore::new(&dvc_ctx, &swc_ctx)?;

        let dvc_ctx = Arc::new(Mutex::new(dvc_ctx));
        let swc_ctx = Arc::new(Mutex::new(swc_ctx));
        let rsc_sto = Arc::new(Mutex::new(rsc_sto));
        let frm_ctxs = (0..Self::FRAMES_IN_FLIGHT)
            .map(|_| FrameContext::new(dvc_ctx.clone(), swc_ctx.clone(), rsc_sto.clone()))
            .collect::<Result<Vec<FrameContext>>>()?;

        Ok(Self {
            dvc_ctx,
            swc_ctx,
            frm_ctxs,
            rsc_sto,
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
            PresentResult::ResizeRequested => {
                self.request_resize();
            }
            PresentResult::Success => {}
        }

        // Increment the frame counter
        self.frame_number += 1;

        Ok(())
    }

    pub fn request_resize(&mut self) {
        self.resize_requested = true;
    }

    fn update_scene<'a>(&mut self, cam: &'a Camera) -> Result<FrameRenderPacket<'a>> {
        let target_size = self.swc_ctx.lock().eyre()?.get_size();
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

    fn get_current_frame(&mut self) -> &mut FrameContext {
        let idx = self.get_current_frame_index();
        &mut self.frm_ctxs[idx]
    }

    fn get_current_frame_index(&self) -> usize {
        (self.frame_number % Self::FRAMES_IN_FLIGHT) as usize
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.dvc_ctx.lock().unwrap().wait_device_idle().unwrap();
    }
}

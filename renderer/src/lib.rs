pub use glam;
pub use winit;

mod camera;
mod contexts;
mod resource_store;
mod utils;

pub use camera::Camera;

use crate::contexts::frame_context::packet::{
    FramePresentPacket, FrameRenderMetadata, FrameRenderPacket, FrameRenderPayload,
};
use crate::contexts::frame_context::{FrameContext};
use crate::contexts::swapchain_context::{SwapchainContext, SwapchainPresentError};
use crate::resource_store::ResourceStore;
use crate::utils::GuardResultExt;
use ash::vk;
use color_eyre::Result;
use contexts::device_context::DeviceContext;
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RendererError {
    #[error("Swapchain is suboptimal and needs to be resized")]
    SwapchainSuboptimal,

    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

pub struct Renderer {
    dvc_ctx: Arc<Mutex<DeviceContext>>,
    swc_ctx: Arc<Mutex<SwapchainContext>>,
    frm_ctxs: Vec<FrameContext>,
    rsc_sto: Arc<Mutex<ResourceStore>>,

    frame_number: u64,
}

impl Renderer {
    const FRAMES_IN_FLIGHT: u64 = 2;

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
        })
    }

    pub fn render_frame(&mut self, cam: &Camera) -> core::result::Result<(), RendererError> {
        // Update the scene and prepare the frame packet
        let render_pkt = self.update_scene(cam);

        // Record and submit the commands for the current frame
        let present_pkt = self.get_current_frame().render(render_pkt).unwrap();
        let swapchain_suboptimal = present_pkt.texture.suboptimal;

        // Present the frame
        let result = match self.get_current_frame().present(present_pkt) {
            Err(SwapchainPresentError::SwapchainSuboptimal) => {
                Err(RendererError::SwapchainSuboptimal)
            }
            Err(SwapchainPresentError::Vulkan(err)) => Err(RendererError::Vulkan(err)),
            Ok(()) => {
                if swapchain_suboptimal {
                    Err(RendererError::SwapchainSuboptimal)
                } else {
                    Ok(())
                }
            }
        };

        // Increment the frame counter
        self.frame_number += 1;

        result
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) -> Result<()> {
        let dvc_ctx = self.dvc_ctx.lock().eyre()?;
        self.swc_ctx.lock().eyre()?.resize(&size, &dvc_ctx)
    }

    fn update_scene<'a>(&mut self, cam: &'a Camera) -> FrameRenderPacket<'a> {
        let target_size = self.swc_ctx.lock().eyre().unwrap().get_size();
        let frame_metadata = FrameRenderMetadata {
            frame_index: self.get_current_frame_index(),
            target_size,
        };
        FrameRenderPacket {
            payload: FrameRenderPayload { cam },
            metadata: frame_metadata,
        }
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

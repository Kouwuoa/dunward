use crate::renderer::resource_store::ResourceStore;
pub use glam;
pub use winit;

mod contexts;
mod resource_store;
mod utils;

use crate::Camera;
use crate::renderer::contexts::frame_context::FrameContext;
use crate::renderer::contexts::frame_context::packet::FrameRenderPacket;
use crate::renderer::contexts::swapchain_context::{SwapchainContext, SwapchainPresentError};
use ash::vk;
use color_eyre::Result;
use contexts::device_context::DeviceContext;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RendererError {
    #[error("Swapchain is suboptimal and needs to be resized")]
    SwapchainSuboptimal,

    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

pub struct Renderer {
    dvc_ctx: DeviceContext,
    swc_ctx: SwapchainContext,
    frm_ctxs: Vec<FrameContext>,
    rsc_sto: ResourceStore,

    frame_number: u64,
}

impl Renderer {
    const FRAMES_IN_FLIGHT: u64 = 2;

    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let _ = color_eyre::install();
        let _ = env_logger::try_init();

        let dvc_ctx = DeviceContext::new(window)?;
        let swc_ctx = dvc_ctx.create_swapchain_context(window)?;
        let mut rsc_sto = ResourceStore::new(&dvc_ctx, &swc_ctx)?;

        let frm_ctxs = (0..Self::FRAMES_IN_FLIGHT)
            .map(|_| FrameContext::new(&dvc_ctx, &swc_ctx, &mut rsc_sto))
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
        // Update the scene based external parameters and prepare the render packet
        let render_pkt = self.update_scene(cam);

        // Get current frame
        let current_frame_index = self.get_current_frame_index();
        let current_frame = &mut self.frm_ctxs[current_frame_index];

        // Render the scene for the current frame
        let present_pkt = current_frame
            .render(render_pkt, &self.dvc_ctx, &self.swc_ctx, &self.rsc_sto)
            .unwrap();

        // Present the frame
        let swapchain_suboptimal = present_pkt.texture.suboptimal;
        let result = match current_frame.present(present_pkt, &self.swc_ctx) {
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
        self.swc_ctx.resize(&size, &self.dvc_ctx)
    }

    fn update_scene<'a>(&mut self, cam: &'a Camera) -> FrameRenderPacket<'a> {
        let target_size = self.swc_ctx.get_size();
        FrameRenderPacket {
            camera: cam,
            frame_index: self.get_current_frame_index(),
            target_size,
        }
    }

    fn get_current_frame_index(&self) -> usize {
        (self.frame_number % Self::FRAMES_IN_FLIGHT) as usize
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.dvc_ctx.wait_device_idle().unwrap();

        // Destroy all frames
        for frame in self.frm_ctxs.drain(..) {
            frame.destroy(&mut self.dvc_ctx).unwrap();
        }
    }
}

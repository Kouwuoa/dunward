pub use glam;
use std::time::Instant;
pub use winit;

mod contexts;
mod subsystems;
mod utils;

use crate::Camera;
use crate::renderer::contexts::frame_context::FrameContext;
use crate::renderer::contexts::frame_context::packet::FrameRenderPacket;
use crate::renderer::contexts::swapchain_context::{SwapchainContext, SwapchainPresentError};
use crate::renderer::subsystems::command_subsystem::CommandSubsystem;
use crate::renderer::subsystems::resource_subsystem::ResourceSubsystem;
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
    // Contexts
    dvc_ctx: DeviceContext,
    swc_ctx: SwapchainContext,
    frm_ctxs: Vec<FrameContext>,

    // Subsystems
    cmd_sys: CommandSubsystem,
    rsc_sys: ResourceSubsystem,

    frame_number: u64,
    time_start: Instant,
}

impl Renderer {
    const FRAMES_IN_FLIGHT: u64 = 3;

    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let _ = color_eyre::install();
        let _ = env_logger::try_init();

        let mut dvc_ctx = DeviceContext::new(window)?;
        let mut cmd_sys = CommandSubsystem::new(&dvc_ctx)?;
        let mut rsc_sys = ResourceSubsystem::new(&dvc_ctx, &cmd_sys)?;
        let swc_ctx = dvc_ctx.create_swapchain_context(window, &rsc_sys)?;
        let frm_ctxs = (0..Self::FRAMES_IN_FLIGHT)
            .map(|_| FrameContext::new(&mut dvc_ctx, &swc_ctx, &mut cmd_sys, &mut rsc_sys))
            .collect::<Result<Vec<FrameContext>>>()?;

        Ok(Self {
            dvc_ctx,
            swc_ctx,
            frm_ctxs,
            cmd_sys,
            rsc_sys,
            frame_number: 0,
            time_start: Instant::now(),
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
            .render(render_pkt, &self.dvc_ctx, &self.swc_ctx, &self.rsc_sys)
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
        // Resize the swapchain
        self.swc_ctx.resize(&size, &self.dvc_ctx)?;

        // Resize the frame contexts
        for frame in &mut self.frm_ctxs {
            frame.resize(&size, &self.rsc_sys);
        }

        Ok(())
    }

    fn update_scene<'a>(&mut self, cam: &'a Camera) -> FrameRenderPacket<'a> {
        let target_size = self.swc_ctx.get_size();
        FrameRenderPacket {
            camera: cam,
            target_size,
            frame_index: self.get_current_frame_index(),
            frame_number: self.frame_number,
            time_start: self.time_start,
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
            frame.destroy(&mut self.cmd_sys).unwrap();
        }
    }
}

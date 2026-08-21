//! Top-level Vulkan Renderer engine orchestrator.
//!
//! Exposes [`Renderer`], orchestrating multi-buffered frames in flight, device initialization,
//! display presentation, resizing, and teardown.

use std::sync::Arc;
use std::time::Instant;

use ash::vk;
use color_eyre::Result;
use thiserror::Error;

use crate::Camera;
use crate::commands::allocator::{CommandRecorderAllocator, CommandRecorderAllocatorExt};
use crate::core::{DeviceContext, device, instance};
use crate::display::{Display, DisplayPresentError};
use crate::frame::Frame;
use crate::frame::packet::FrameRenderPacket;
use crate::resources::create_memory_allocator;
use crate::resources::factory::ResourceFactory;
use crate::resources::store::ResourceStore;

#[derive(Debug, Error)]
pub enum RendererError {
    #[error("Display surface is suboptimal and needs to be resized")]
    DisplaySuboptimal,

    #[error("Swapchain is suboptimal and needs to be resized")]
    SwapchainSuboptimal,

    #[error("Vulkan error: {0}")]
    Vulkan(#[from] vk::Result),
}

pub struct Renderer {
    instance: instance::Instance,
    device: Arc<device::Device>,

    // Hardware & Presentation
    display: Display,
    frames: Vec<Frame>,

    // Commands & Resources
    cmd_allocator: CommandRecorderAllocator,
    resource_factory: ResourceFactory,
    resource_store: ResourceStore,

    frame_number: u64,
    time_start: Instant,
}

impl Renderer {
    const FRAMES_IN_FLIGHT: u64 = 3;

    pub fn new(window: &winit::window::Window) -> Result<Self> {
        let _ = color_eyre::install();
        let _ = env_logger::try_init();

        let instance = instance::Instance::new(Some(window))?;
        let mut surface = instance.create_surface(window)?;
        let device = Arc::new(instance.create_device(&surface)?);
        surface.init_surface_formats(&device)?;
        surface.init_surface_present_modes(&device)?;
        let display = Display::new(window, device.clone(), surface)?;

        let mut cmd_allocator =
            CommandRecorderAllocator::new(dvc_ctx.logical_device_handle())?;
        let memory_allocator = create_memory_allocator(&dvc_ctx)?;
        let resource_factory = ResourceFactory::new(
            memory_allocator.clone(),
            dvc_ctx.logical_device_handle(),
        )?;
        let mut resource_store = ResourceStore::new(&resource_factory)?;
        let nearest_sampler = dvc_ctx.create_vk_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::NEAREST)
                .min_filter(vk::Filter::NEAREST)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT),
        )?;
        resource_store.add_sampler(nearest_sampler);

        let display_ctx = dvc_ctx.create_display_context(window, memory_allocator)?;
        let frm_ctxs = (0..Self::FRAMES_IN_FLIGHT)
            .map(|_| {
                Frame::new(
                    &mut dvc_ctx,
                    &display_ctx,
                    &mut cmd_allocator,
                    &resource_factory,
                    &mut resource_store,
                )
            })
            .collect::<Result<Vec<Frame>>>()?;

        Ok(Self {
            dvc_ctx,
            display_ctx,
            frm_ctxs,
            cmd_allocator,
            transfer_recorder,
            resource_factory,
            resource_store,
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
            .render(render_pkt, &self.dvc_ctx, &self.display_ctx)
            .unwrap();

        // Present the frame
        let display_suboptimal = present_pkt.texture.suboptimal;
        let result = match current_frame.present(present_pkt, &self.display_ctx) {
            Err(DisplayPresentError::DisplaySuboptimal) => {
                Err(RendererError::DisplaySuboptimal)
            }
            Err(DisplayPresentError::Vulkan(err)) => Err(RendererError::Vulkan(err)),
            Ok(()) => {
                if display_suboptimal {
                    Err(RendererError::DisplaySuboptimal)
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
        // Resize the display surface
        self.display_ctx.resize(&size, &self.dvc_ctx)?;

        // Resize the frame contexts
        for frame in &mut self.frm_ctxs {
            frame.resize(&size, &self.resource_factory);
        }

        Ok(())
    }

    fn update_scene<'a>(&mut self, cam: &'a Camera) -> FrameRenderPacket<'a> {
        let target_size = self.display_ctx.get_size();
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
            frame.destroy(&mut self.cmd_allocator).unwrap();
        }
    }
}

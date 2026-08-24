//! Top-level Vulkan Renderer engine orchestrator.
//!
//! Exposes [`Renderer`], orchestrating multi-buffered frames in flight, device initialization,
//! display presentation, resizing, and teardown.

use std::sync::Arc;
use std::time::Instant;

use ash::vk;
use color_eyre::Result;
use thiserror::Error;

use crate::gpu::Gpu;
use crate::{
    Camera,
    commands::allocator::{CommandRecorderAllocator, CommandRecorderAllocatorExt},
    display::{Display, DisplayPresentError},
    frame::Frame,
    frame::packet::FrameRenderPacket,
    resources::factory::ResourceFactory,
    resources::store::ResourceStore,
};

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
    gpu: Arc<Gpu>,

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

        let (gpu, mut surface) = Gpu::new(window)?;
        surface.init_surface_formats(&gpu)?;
        surface.init_surface_present_modes(&gpu)?;
        let gpu = Arc::new(gpu);
        let display = Display::new(window, gpu.clone(), surface)?;

        let mut cmd_allocator = CommandRecorderAllocator::new(gpu.raw_logical())?;
        let resource_factory = ResourceFactory::new(gpu.clone())?;
        let mut resource_store = ResourceStore::new(&resource_factory)?;
        let nearest_sampler = gpu.create_vk_sampler(
            &vk::SamplerCreateInfo::default()
                .mag_filter(vk::Filter::NEAREST)
                .min_filter(vk::Filter::NEAREST)
                .mipmap_mode(vk::SamplerMipmapMode::NEAREST)
                .address_mode_u(vk::SamplerAddressMode::REPEAT)
                .address_mode_v(vk::SamplerAddressMode::REPEAT)
                .address_mode_w(vk::SamplerAddressMode::REPEAT),
        )?;
        resource_store.add_sampler(nearest_sampler);

        let frames = (0..Self::FRAMES_IN_FLIGHT)
            .map(|_| {
                Frame::new(
                    gpu.clone(),
                    &display,
                    &mut cmd_allocator,
                    &resource_factory,
                    &mut resource_store,
                )
            })
            .collect::<Result<Vec<Frame>>>()?;

        Ok(Self {
            gpu,
            display,
            frames,
            cmd_allocator,
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
        let current_frame = &mut self.frames[current_frame_index];

        // Render the scene for the current frame
        let present_pkt = current_frame
            .render(render_pkt, self.gpu.clone(), &self.display)
            .unwrap();

        // Present the frame
        let display_suboptimal = present_pkt.texture.suboptimal;
        let result = match current_frame.present(present_pkt, &self.display) {
            Err(DisplayPresentError::DisplaySuboptimal) => Err(RendererError::DisplaySuboptimal),
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
        self.display.resize(&size, &self.gpu)?;

        // Resize the frame contexts
        for frame in &mut self.frames {
            frame.resize(&size, &self.resource_factory);
        }

        Ok(())
    }

    fn update_scene<'a>(&mut self, cam: &'a Camera) -> FrameRenderPacket<'a> {
        let target_size = self.display.get_size();
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
        self.gpu.wait_idle().unwrap();

        // Destroy all frames
        for frame in self.frames.drain(..) {
            frame.destroy(&mut self.cmd_allocator).unwrap();
        }
    }
}

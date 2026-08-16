//! Core Vulkan Hardware Abstraction Layer (HAL).
//!
//! Exposes [`DeviceContext`], which manages the lifetime of the Vulkan instance,
//! logical device, physical device selection, queue families, and synchronization primitives.

pub(crate) mod device;
pub(crate) mod instance;
pub(crate) mod queue;
pub(crate) mod semaphore;
pub(crate) mod surface;

pub(crate) use queue::Queue;
pub(crate) use semaphore::{BinarySemaphore, SignalSemaphore, TimelineSemaphore, WaitSemaphore};

use std::sync::{Arc, Mutex};
use std::time::Duration;

use ash::vk;
use color_eyre::Result;

use crate::commands::recorder::{CommandRecorder, Executable, Idle};
use crate::display::DisplayContext;

/// Main abstraction around the graphics API context for rendering.
pub(crate) struct DeviceContext {
    instance: instance::Instance,
    device: device::Device,
    surface: surface::Surface,
}

impl DeviceContext {
    pub(crate) fn new(window: &winit::window::Window) -> Result<Self> {
        log::info!("Creating DeviceContext");

        let instance = instance::Instance::new(Some(window))?;
        let mut surface = instance.create_surface(window)?;
        let device = instance.create_device(&surface)?;
        let _ = surface.generate_surface_formats(&device)?;
        let _ = surface.generate_surface_present_modes(&device)?;

        Ok(Self {
            instance,
            device,
            surface,
        })
    }

    pub(crate) fn instance_handle(&self) -> ash::Instance {
        self.instance.inner().clone()
    }

    pub(crate) fn logical_device_handle(&self) -> Arc<ash::Device> {
        self.device.logical.clone()
    }

    pub(crate) fn physical_device_handle(&self) -> vk::PhysicalDevice {
        self.device.physical
    }

    pub(crate) fn get_graphics_queue(&self) -> Arc<Queue> {
        self.device.get_graphics_queue()
    }

    pub(crate) fn get_present_queue(&self) -> Arc<Queue> {
        self.device.get_present_queue()
    }

    pub(crate) fn get_compute_queue(&self) -> Arc<Queue> {
        self.device.get_compute_queue()
    }

    pub(crate) fn get_transfer_queue(&self) -> Arc<Queue> {
        self.device.get_transfer_queue()
    }

    pub(crate) fn create_binary_semaphore(&self) -> Result<BinarySemaphore> {
        Ok(BinarySemaphore::new(unsafe {
            self.device
                .logical
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)?
        }))
    }

    pub(crate) fn create_timeline_semaphore(&self) -> Result<TimelineSemaphore> {
        Ok(TimelineSemaphore::new(unsafe {
            self.device.logical.create_semaphore(
                &vk::SemaphoreCreateInfo::default().push_next(&mut vk::SemaphoreTypeCreateInfo {
                    semaphore_type: vk::SemaphoreType::TIMELINE,
                    initial_value: 0,
                    ..Default::default()
                }),
                None,
            )?
        }))
    }

    pub(crate) fn wait_timeline_semaphore(
        &self,
        semaphore: vk::Semaphore,
        value: u64,
        timeout: Duration,
    ) -> Result<()> {
        let wait_info = vk::SemaphoreWaitInfo::default()
            .semaphores(std::slice::from_ref(&semaphore))
            .values(std::slice::from_ref(&value));

        unsafe {
            self.device
                .logical
                .wait_semaphores(&wait_info, timeout.as_nanos() as u64)?;
        }
        Ok(())
    }

    pub(crate) fn wait_and_reset_fence(&self, fence: vk::Fence, timeout: Duration) -> Result<()> {
        unsafe {
            let fences = [fence];
            self.device
                .logical
                .wait_for_fences(&fences, true, timeout.as_nanos() as u64)?;
            self.device.logical.reset_fences(&fences)?;
        }
        Ok(())
    }

    pub(crate) fn wait_device_idle(&self) -> Result<()> {
        Ok(unsafe { self.device.logical.device_wait_idle()? })
    }

    pub(crate) fn create_display_context(
        &self,
        window: &winit::window::Window,
        memory_allocator: Arc<Mutex<vk_mem::Allocator>>,
    ) -> Result<DisplayContext> {
        DisplayContext::new(window, self, memory_allocator)
    }

    pub(crate) fn submit(
        &self,
        cmd_recorder: CommandRecorder<Executable>,
        wait_semaphores: &[WaitSemaphore],
        signal_semaphores: &[SignalSemaphore],
        fence: Option<vk::Fence>,
    ) -> Result<CommandRecorder<Idle>> {
        let cmd = cmd_recorder.get_command_buffer();
        let queue = cmd_recorder.get_queue();

        self.device
            .submit(cmd, queue, wait_semaphores, signal_semaphores, fence)?;

        Ok(CommandRecorder::<Idle>::new_from_old(cmd_recorder))
    }

    pub(crate) fn create_vk_image_view(&self, info: &vk::ImageViewCreateInfo) -> Result<vk::ImageView> {
        Ok(unsafe { self.device.logical.create_image_view(info, None)? })
    }

    pub(crate) fn create_vk_sampler(&self, info: &vk::SamplerCreateInfo) -> Result<vk::Sampler> {
        Ok(unsafe { self.device.logical.create_sampler(info, None)? })
    }

    pub(crate) fn create_vk_fence(&self, info: &vk::FenceCreateInfo) -> Result<vk::Fence> {
        Ok(unsafe { self.device.logical.create_fence(info, None)? })
    }

    pub(crate) fn create_vk_swapchain_loader(&self) -> ash::khr::swapchain::Device {
        ash::khr::swapchain::Device::new(self.instance.inner(), &self.device.logical)
    }
}

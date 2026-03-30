pub(crate) mod queue;

mod device;
mod instance;
mod surface;

use crate::renderer::contexts::device_context::queue::Queue;
use crate::renderer::contexts::swapchain_context::SwapchainContext;
use crate::renderer::subsystems::command_subsystem::command_recorder::{
    CommandRecorder, Executable, Idle,
};
use crate::renderer::subsystems::resource_subsystem::ResourceSubsystem;
use ash::vk;
use color_eyre::Result;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Main abstraction around the graphics API context for rendering.
pub(crate) struct DeviceContext {
    instance: instance::Instance,
    device: device::Device,
    surface: surface::Surface,
}

impl DeviceContext {
    pub fn new(window: &winit::window::Window) -> Result<Self> {
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

    pub fn instance_handle(&self) -> ash::Instance {
        self.instance.inner().clone()
    }

    pub fn logical_device_handle(&self) -> Arc<ash::Device> {
        self.device.logical.clone()
    }

    pub fn physical_device_handle(&self) -> vk::PhysicalDevice {
        self.device.physical
    }

    pub fn get_graphics_queue(&self) -> Arc<Queue> {
        self.device.get_graphics_queue()
    }

    pub fn get_present_queue(&self) -> Arc<Queue> {
        self.device.get_present_queue()
    }

    pub fn get_compute_queue(&self) -> Arc<Queue> {
        self.device.get_compute_queue()
    }

    pub fn get_transfer_queue(&self) -> Arc<Queue> {
        self.device.get_transfer_queue()
    }

    pub fn wait_timeline_semaphore(
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

    pub fn wait_and_reset_fence(&self, fence: vk::Fence, timeout: Duration) -> Result<()> {
        unsafe {
            let fences = [fence];
            self.device
                .logical
                .wait_for_fences(&fences, true, timeout.as_nanos() as u64)?;
            self.device.logical.reset_fences(&fences)?;
        }
        Ok(())
    }

    pub fn wait_device_idle(&self) -> Result<()> {
        Ok(unsafe { self.device.logical.device_wait_idle()? })
    }

    pub fn create_swapchain_context(
        &self,
        window: &winit::window::Window,
        rsc_sys: &ResourceSubsystem,
    ) -> Result<SwapchainContext> {
        SwapchainContext::new(window, self, rsc_sys)
    }

    pub fn submit(
        &self,
        cmd_recorder: CommandRecorder<Executable>,
        wait_semaphore: &SemaphoreSubmitInfo,
        signal_semaphore: &SemaphoreSubmitInfo,
        fence: vk::Fence,
    ) -> Result<CommandRecorder<Idle>> {
        let cmd = cmd_recorder.get_command_buffer();
        let queue = cmd_recorder.get_queue();

        self.device
            .submit(cmd, queue, wait_semaphores, signal_semaphores, fence, None)?;

        Ok(CommandRecorder::<Idle>::new_from_old(cmd_recorder))
    }

    pub fn submit_with_timeline(
        &self,
        cmd_recorder: CommandRecorder<Executable>,
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
        fence: vk::Fence,
        timeline_semaphore_
    ) -> Result<CommandRecorder<Idle>> {
        let cmd = cmd_recorder.get_command_buffer();
        let queue = cmd_recorder.get_queue();

        self.device.submit(
            cmd,
            queue,
            wait_semaphores,
            signal_semaphores,
            fence,
            timeline_semaphore_info,
        )?;

        Ok(CommandRecorder::<Idle>::new_from_old(cmd_recorder))
    }

    pub fn create_vk_image_view(&self, info: &vk::ImageViewCreateInfo) -> Result<vk::ImageView> {
        Ok(unsafe { self.device.logical.create_image_view(info, None)? })
    }

    pub fn create_vk_sampler(&self, info: &vk::SamplerCreateInfo) -> Result<vk::Sampler> {
        Ok(unsafe { self.device.logical.create_sampler(info, None)? })
    }

    pub fn create_vk_semaphore(&self, info: &vk::SemaphoreCreateInfo) -> Result<vk::Semaphore> {
        Ok(unsafe { self.device.logical.create_semaphore(info, None)? })
    }

    pub fn create_vk_fence(&self, info: &vk::FenceCreateInfo) -> Result<vk::Fence> {
        Ok(unsafe { self.device.logical.create_fence(info, None)? })
    }

    pub fn create_vk_swapchain_loader(&self) -> ash::khr::swapchain::Device {
        ash::khr::swapchain::Device::new(self.instance.inner(), &self.device.logical)
    }
}

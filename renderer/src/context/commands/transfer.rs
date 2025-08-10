use super::super::queue::Queue;
use ash::vk;
use color_eyre::eyre::Result;
use std::sync::Arc;

/// This is a specialized command encoder designed for performing on-off GPU transfer operations.
///
/// This struct manages its own Vulkan fence, command pool, and command buffer
/// to submit immediate commands to the GPU transfer queue. It is useful for:
///
/// - Uploading buffers or textures outside the main render loop.
/// - Running GPU compute tasks from background threads without complex synchronization.
/// - Executing short, synchronous GPU commands that must complete immediately.
///
/// # Safety
///
/// The `TransferCommandEncoder` assumes exclusive access to its internal command buffer.
/// It is not thread-safe to call `immediate_submit` concurrently.
/// The queue passed in the constructor must support transfer operations.
pub(crate) struct TransferCommandEncoder {
    transfer_fence: vk::Fence,
    command_pool: vk::CommandPool,
    command_buffer: vk::CommandBuffer,

    transfer_queue: Arc<Queue>,
    device: Arc<ash::Device>,
}

impl TransferCommandEncoder {
    pub fn new(transfer_queue: Arc<Queue>, device: Arc<ash::Device>) -> Result<Self> {
        let transfer_fence_info = vk::FenceCreateInfo::default();
        let transfer_fence = unsafe { device.create_fence(&transfer_fence_info, None)? };

        let command_pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(transfer_queue.family.index)
            // Allow the pool to reset individual command buffers
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe { device.create_command_pool(&command_pool_info, None)? };

        let command_buffer_info = vk::CommandBufferAllocateInfo::default()
            .command_pool(command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffer = unsafe { device.allocate_command_buffers(&command_buffer_info)?[0] };

        Ok(Self {
            transfer_fence,
            command_pool,
            command_buffer,
            transfer_queue,
            device,
        })
    }

    // Instantly execute some commands to the GPU without dealing with the render loop and other synchronization
    // This is great for compute calculations and can be used from a background thread separated from the render loop
    pub fn immediate_submit<F>(&self, func: F) -> Result<()>
    where
        F: FnOnce(vk::CommandBuffer, &ash::Device) -> Result<()>,
    {
        let cmd = self.command_buffer;

        // This command buffer will be used exactly once before resetting
        let cmd_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        // Begin the command buffer recording
        unsafe {
            self.device.begin_command_buffer(cmd, &cmd_begin_info)?;
        }

        func(cmd, &*self.device)?;

        // End the command buffer recording
        unsafe {
            self.device.end_command_buffer(cmd)?;
        }

        // Submit command buffer to the queue and execute it
        let cmd = [cmd];
        let submit = vk::SubmitInfo::default()
            .wait_semaphores(&[])
            .wait_dst_stage_mask(&[])
            .command_buffers(&cmd)
            .signal_semaphores(&[]);
        unsafe {
            self.device
                .queue_submit(self.transfer_queue.handle, &[submit], self.transfer_fence)?;
        }

        unsafe {
            // `transfer_fence` will now block until the commands finish execution
            self.device
                .wait_for_fences(&[self.transfer_fence], true, 9999999999)?;
            self.device.reset_fences(&[self.transfer_fence])?;
            // Reset command buffers inside command pool
            self.device
                .reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty())?;
        }

        Ok(())
    }
}

impl Drop for TransferCommandEncoder {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_command_pool(self.command_pool, None);
            self.device.destroy_fence(self.transfer_fence, None);
        }
    }
}

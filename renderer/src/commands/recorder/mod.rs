//! Vulkan command buffer recorder and typestate lifecycle state machine.
//!
//! Provides [`CommandRecorder`] parameterized by typestate markers:
//! - [`Idle`]: Freshly allocated or reset, ready to begin recording.
//! - [`Recording`]: Currently recording commands (draws, dispatches, barriers, transfers).
//! - [`Executable`]: Closed and ready for submission to a [`crate::core::queue::Queue`].

mod barriers;
mod pipeline;
mod transfers;

use std::marker::PhantomData;
use std::sync::Arc;

use ash::vk;
use color_eyre::Result;

use crate::commands::allocator::CommandRecorderAllocator;
use crate::core::queue::Queue;

/// Ready to record
pub(crate) struct Idle;
/// Currently recording
pub(crate) struct Recording;
/// Recording finished; can submit
pub(crate) struct Executable;

pub(crate) struct CommandRecorder<State> {
    pub(crate) command_buffer: vk::CommandBuffer,
    pub(crate) queue: Arc<Queue>,
    pub(crate) device: Arc<ash::Device>,
    /// Note that this is only an `Option` to allow for the allocator to be dropped.
    pub(crate) allocator: Option<CommandRecorderAllocator>,
    pub(crate) _state: PhantomData<State>,
}

impl<State> CommandRecorder<State> {
    pub(crate) fn get_queue(&self) -> Arc<Queue> {
        self.queue.clone()
    }
    pub(crate) fn get_command_buffer(&self) -> vk::CommandBuffer {
        self.command_buffer
    }
}

impl CommandRecorder<Idle> {
    pub(crate) fn new(
        command_buffer: vk::CommandBuffer,
        queue: Arc<Queue>,
        device: Arc<ash::Device>,
        allocator: CommandRecorderAllocator,
    ) -> Self {
        Self {
            command_buffer,
            queue,
            device,
            allocator: Some(allocator),
            _state: PhantomData,
        }
    }

    pub(crate) fn new_from_old(old: CommandRecorder<Executable>) -> Self {
        Self {
            command_buffer: old.command_buffer,
            queue: old.queue,
            device: old.device,
            allocator: old.allocator,
            _state: PhantomData,
        }
    }

    pub(crate) fn record<F>(self, f: F) -> Result<CommandRecorder<Executable>>
    where
        F: FnOnce(&CommandRecorder<Recording>) -> Result<()>,
    {
        let recorder = self.begin_recording()?;

        let result = f(&recorder);

        let recorder = recorder.end_recording();

        // Propagate user error first
        result?;

        recorder
    }

    fn begin_recording(self) -> Result<CommandRecorder<Recording>> {
        let begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        unsafe {
            self.device
                .begin_command_buffer(self.command_buffer, &begin_info)?;
        }

        Ok(CommandRecorder::<Recording> {
            command_buffer: self.command_buffer,
            queue: self.queue,
            device: self.device,
            allocator: self.allocator,
            _state: PhantomData,
        })
    }
}

impl CommandRecorder<Recording> {
    fn end_recording(self) -> Result<CommandRecorder<Executable>> {
        unsafe { self.device.end_command_buffer(self.command_buffer)? }
        Ok(CommandRecorder::<Executable> {
            command_buffer: self.command_buffer,
            queue: self.queue,
            device: self.device,
            allocator: self.allocator,
            _state: PhantomData,
        })
    }
}

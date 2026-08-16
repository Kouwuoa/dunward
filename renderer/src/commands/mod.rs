//! Vulkan Command Buffers, Allocators, and Execution Recorders.
//!
//! Exposes [`CommandSubsystem`] for managing command allocation infrastructure,
//! [`CommandRecorder`] for recording GPU commands, and [`TransferCommandRecorder`] for immediate transfers.

pub mod allocator;
pub mod recorder;
pub mod transfer;

pub use allocator::CommandRecorderAllocator;
pub use recorder::{CommandRecorder, Executable, Idle, Recording};
pub use transfer::TransferCommandRecorder;

use crate::commands::allocator::CommandRecorderAllocatorExt;
use crate::core::DeviceContext;
use color_eyre::eyre::Result;
use std::sync::Arc;

pub struct CommandSubsystem {
    pub command_recorder_allocator: CommandRecorderAllocator,
    pub transfer_command_recorder: Arc<TransferCommandRecorder>,
}

impl CommandSubsystem {
    pub fn new(dvc_ctx: &DeviceContext) -> Result<Self> {
        let command_recorder_allocator =
            CommandRecorderAllocator::new(dvc_ctx.logical_device_handle())?;
        let transfer_command_recorder = Arc::new(TransferCommandRecorder::new(
            dvc_ctx.get_transfer_queue(),
            dvc_ctx.logical_device_handle(),
        )?);

        Ok(Self {
            command_recorder_allocator,
            transfer_command_recorder,
        })
    }
}

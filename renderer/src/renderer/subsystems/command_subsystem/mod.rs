mod command_recorder;
mod command_recorder_allocator;
mod transfer_command_recorder;

use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::subsystems::command_subsystem::command_recorder_allocator::{
    CommandRecorderAllocator, CommandRecorderAllocatorExt,
};
use crate::renderer::subsystems::command_subsystem::transfer_command_recorder::TransferCommandRecorder;
use color_eyre::Result;
use std::sync::Arc;

pub struct CommandSubsystem {
    command_recorder_allocator: CommandRecorderAllocator,
    transfer_command_recorder: Arc<TransferCommandRecorder>,
}

impl CommandSubsystem {
    pub fn new(dvc: &DeviceContext) -> Result<Self> {
        let command_recorder_allocator =
            CommandRecorderAllocator::new(dvc.logical_device_handle())?;
        let transfer_command_recorder = Arc::new(TransferCommandRecorder::new(
            dvc.get_transfer_queue(),
            dvc.logical_device_handle(),
        )?);

        Ok(Self {
            command_recorder_allocator,
            transfer_command_recorder,
        })
    }
}

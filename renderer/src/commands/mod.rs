//! Vulkan Command Buffers, Allocators, and Execution Recorders.
//!
//! Exposes [`CommandRecorderAllocator`] for command pool and buffer management,
//! [`CommandRecorder`] for recording GPU commands, and [`TransferCommandRecorder`] for synchronous GPU uploads.

pub(crate) mod allocator;
pub(crate) mod recorder;
pub(crate) mod transfer;

pub(crate) use allocator::CommandRecorderAllocator;
pub(crate) use recorder::{CommandRecorder, Executable, Idle, Recording};
pub(crate) use transfer::TransferCommandRecorder;

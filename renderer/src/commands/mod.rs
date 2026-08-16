//! Vulkan Command Buffers, Allocators, and Execution Recorders.
//!
//! Exposes [`CommandRecorderAllocator`] for command pool and buffer management,
//! [`CommandRecorder`] for recording GPU commands, and [`TransferCommandRecorder`] for synchronous GPU uploads.

pub mod allocator;
pub mod recorder;
pub mod transfer;

pub use allocator::CommandRecorderAllocator;
pub use recorder::{CommandRecorder, Executable, Idle, Recording};
pub use transfer::TransferCommandRecorder;

//! Core Vulkan Hardware Abstraction Layer (HAL).
//!
//! Exposes [`DeviceContext`], which manages the lifetime of the Vulkan instance,
//! logical device, physical device selection, queue families, and synchronization primitives.

pub(crate) mod device;
pub(crate) mod instance;
pub(crate) mod queue;
pub(crate) mod semaphore;
pub(crate) mod surface;


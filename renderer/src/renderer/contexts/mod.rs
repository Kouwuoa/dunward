//! Renderer Contexts
//!
//! This module defines long-lived Vulkan-facing context types and their ownership boundaries
//!
//! - `DeviceContext`: Owns Vulkan instance/device/surface/queues and low-level submission helpers
//! - `SwapchainContext`: Owns swapchain images and present/acquire flow
//! - `FrameContext`: Owns per-frame synchronization and command recording state
//!
//! Design rules:
//! - Contexts own foundational handles/state
//! - Subsystems build higher-level behavior on top of contexts
//! - Prefer passing narrow references (`&DeviceContext`) over exposing raw fields

pub mod device_context;
pub mod frame_context;
pub mod swapchain_context;

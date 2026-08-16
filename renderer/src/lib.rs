//! Dunward Custom Vulkan Graphics Library.
//!
//! Organizes GPU rendering into domain modules:
//! - [`core`]: Low-level Vulkan instance, device context, queues, semaphores, and surfaces.
//! - [`swapchain`]: Swapchain management, surface formats, and image acquisition/presentation.
//! - [`commands`]: Command pools, synchronous transfer recorders, and typestate command recorders.
//! - [`resources`]: Megabuffers, memory allocations, textures, and descriptor set management.
//! - [`pipeline`]: Shader module compilation, pipeline builders, and material abstractions.
//! - [`scene`]: Geometry primitives, meshes, models, and vertex layout definitions.
//! - [`frame`]: Per-frame synchronization, multi-buffering, and rendering stages.
//! - [`renderer`]: Top-level engine orchestrator ([`Renderer`]).

pub mod camera;
pub mod commands;
pub mod core;
pub mod frame;
pub mod pipeline;
pub mod renderer;
pub mod resources;
pub mod scene;
pub mod swapchain;
pub mod utils;

pub use camera::Camera;
pub use glam;
pub use renderer::{Renderer, RendererError};
pub use winit;

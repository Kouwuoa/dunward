//! Dunward Custom Vulkan Graphics Library.
//!
//! Organizes GPU rendering into domain modules:
//! - [`core`]: Low-level Vulkan instance, device context, queues, semaphores, and surfaces.
//! - [`display`]: Window display surface, presentation modes, and backbuffer management.
//! - [`commands`]: Command pools, synchronous transfer recorders, and typestate command recorders.
//! - [`resources`]: Megabuffers, memory allocations, textures, and descriptor set management.
//! - [`material`]: Material pipeline builders, shader modules, and GPU constant data.
//! - [`scene`]: Geometry primitives, meshes, models, and vertex layout definitions.
//! - [`frame`]: Per-frame synchronization, multi-buffering, and rendering stages.
//! - [`renderer`]: Top-level engine orchestrator ([`Renderer`]).

pub mod camera;
pub mod commands;
pub mod core;
pub mod display;
pub mod frame;
pub mod material;
pub mod renderer;
pub mod resources;
pub mod scene;
pub mod utils;

pub use camera::Camera;
pub use glam;
pub use renderer::{Renderer, RendererError};
pub use winit;

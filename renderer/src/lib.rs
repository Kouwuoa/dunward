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

pub(crate) mod camera;
pub(crate) mod commands;
pub(crate) mod gpu;
pub(crate) mod display;
pub(crate) mod frame;
pub(crate) mod material;
pub(crate) mod renderer;
pub(crate) mod resources;
pub(crate) mod scene;
pub(crate) mod utils;

pub use camera::Camera;
pub use glam;
pub use renderer::{Renderer, RendererError};
pub use winit;

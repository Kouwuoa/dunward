//! Renderer Subsystems
//!
//! This module defines higher-level functional units built on top of renderer contexts
//!
//! - `ResourceSubsystem`: Manages GPU resource creation, uploads, and lifetime-backed storage
//! - `DescriptorSubsystem`: Manages descriptor allocation, writes, and binding policy
//! - `CommandSubsystem`: Manages command recorder allocation/recycling and submission helpers
//!
//! Design rules:
//! - Subsystems own domain behavior and policy, not foundational Vulkan handles
//! - Subsystems depend on contexts (EX: `DeviceContext`) instead of duplicating them
//! - Keep subsystem APIs task-oriented (EX: `upload_mesh`, `allocate_recorder`)

pub(crate) mod command_subsystem;
pub(crate) mod descriptor_subsystem;
pub(crate) mod resource_subsystem;

//! CPU-GPU data structures for material constants, uniform buffers, and push constants.
//!
//! Provides `bytemuck`-compatible `#[repr(C)]` POD structs for per-frame scene data,
//! per-material properties, per-object instance transforms, and per-draw push constants.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};

/// Data unique to each frame passed into uniform buffer
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct PerFrameData {
    pub(crate) viewproj: Mat4,
    pub(crate) near: f32,
    pub(crate) far: f32,
    _padding: [f32; 2],
}

/// Data unique to each material passed as elements into a storage buffer
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct PerMaterialData {
    pub(crate) texture_index: u32,
    pub(crate) sampler_index: u32,
}

/// Data unique to each object passed as elements into a storage buffer
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct PerObjectData {
    pub(crate) model: Mat4,
}

/// Data unique to each vertex passed as elements into a vertex buffer
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct PerVertexData {
    pub(crate) position: Vec3,
    pub(crate) texcoord: Vec2,
}

/// Data unique to each draw call passed as a push constant
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct PerDrawData {
    pub(crate) object_index: u32,
    pub(crate) material_index: u32,
    pub(crate) vertex_offset: u32,
    pub(crate) time_sec: f32,
}

impl PerDrawData {
    pub(crate) fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

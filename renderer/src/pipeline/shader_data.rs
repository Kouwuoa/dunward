//! CPU-GPU data structures for uniform buffers, storage buffers, and push constants.
//!
//! Provides `bytemuck`-compatible `#[repr(C)]` POD structs for per-frame scene data,
//! per-material properties, per-object instance transforms, and per-draw push constants.

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2, Vec3};

/// Data unique to each frame passed into uniform buffer
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct PerFrameData {
    pub viewproj: Mat4,
    pub near: f32,
    pub far: f32,
    _padding: [f32; 2],
}

/// Data unique to each material passed as elements into a storage buffer
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct PerMaterialData {
    pub texture_index: u32,
    pub sampler_index: u32,
}

/// Data unique to each object passed as elements into a storage buffer
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct PerObjectData {
    pub model: Mat4,
}

/// Data unique to each vertex passed as elements into a vertex buffer
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct PerVertexData {
    pub position: Vec3,
    pub texcoord: Vec2,
}

/// Data unique to each draw call passed as a push constant
#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct PerDrawData {
    pub object_index: u32,
    pub material_index: u32,
    pub vertex_offset: u32,
    pub time_sec: f32,
}

impl PerDrawData {
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::bytes_of(self)
    }
}

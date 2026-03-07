use crate::renderer::subsystems::resource_subsystem::resource_types::texture::{
    ColorTexture, StorageTexture, Texture,
};
use ash::vk::Sampler;

/// Coordinates how resources from `ResourceStore` are exposed to shaders.
///
/// `ResourceBinder` is responsible for descriptor-binding policy and updates:
/// - binds long-lived global resources (for example, sampler/texture arrays),
/// - updates per-frame and per-draw bindings (for example, uniform/storage buffers),
/// - keeps binding points aligned with the bindless descriptor set layout.
///
/// It does not create resources (`ResourceFactory`) or own resource lifetimes (`ResourceStore`).
pub(crate) struct ResourceBinder {}

impl ResourceBinder {
    pub(crate) fn bind_storage_textures(&self, textures: &[StorageTexture]) {}
    pub(crate) fn bind_global_sampled_textures(&self, textures: &Vec<ColorTexture>) {}
    pub(crate) fn bind_global_samplers(&self, samplers: &Vec<Sampler>) {}
}

impl ResourceBinder {
    pub fn new() -> Self {
        Self {}
    }
}

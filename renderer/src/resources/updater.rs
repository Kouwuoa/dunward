//! Batched resource descriptor updates during command recording.
//!
//! Exposes [`ResourceUpdater`] and [`ResourceUpdateBuilder`], enabling render stages
//! to queue descriptor set updates (e.g. updating render target storage textures)
//! and commit them prior to shader dispatches.

use ash::vk;

use super::descriptors::DescriptorWriter;
use super::texture::StorageTexture;
use crate::material::Material;

pub(crate) struct ResourceUpdater<'a> {
    device: &'a ash::Device,
    #[allow(dead_code)]
    command_buffer: &'a vk::CommandBuffer,
    updates: Vec<ResourceUpdate<'a>>,
}

impl<'a> ResourceUpdater<'a> {
    pub(crate) fn new(device: &'a ash::Device, command_buffer: &'a vk::CommandBuffer) -> Self {
        Self {
            device,
            command_buffer,
            updates: Vec::new(),
        }
    }

    pub(crate) fn enqueue_update<F>(&mut self, build_update: F, material: &Material)
    where
        F: FnOnce(&mut ResourceUpdateBuilder),
    {
        let mut builder = ResourceUpdateBuilder::new(material);
        build_update(&mut builder);
        self.updates.push(builder.build());
    }

    pub(crate) fn execute_updates(&mut self) {
        for update in self.updates.drain(..) {
            update.execute(self.device);
        }
    }
}

pub(crate) struct ResourceUpdateBuilder<'a> {
    descriptor_writer: DescriptorWriter<'a>,
    descriptor_set: vk::DescriptorSet,
}

impl<'a> ResourceUpdateBuilder<'a> {
    pub(crate) fn new(material: &Material) -> Self {
        let descriptor_writer = DescriptorWriter::default();
        Self {
            descriptor_writer,
            descriptor_set: material.descriptor_set,
        }
    }

    pub(crate) fn set_render_target_texture(&mut self, texture: &StorageTexture) {
        self.descriptor_writer.write_image(
            0,
            texture.view,
            vk::Sampler::null(),
            vk::ImageLayout::GENERAL,
            vk::DescriptorType::STORAGE_IMAGE,
        );
    }

    fn build(self) -> ResourceUpdate<'a> {
        ResourceUpdate {
            descriptor_writer: self.descriptor_writer,
            descriptor_set: self.descriptor_set,
        }
    }
}

struct ResourceUpdate<'a> {
    descriptor_writer: DescriptorWriter<'a>,
    descriptor_set: vk::DescriptorSet,
}

impl<'a> ResourceUpdate<'a> {
    fn execute(mut self, device: &ash::Device) {
        self.descriptor_writer
            .update_set(device, self.descriptor_set);
    }
}

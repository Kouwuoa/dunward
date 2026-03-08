use crate::renderer::subsystems::resource_subsystem::resource_descriptors::descriptor_writer::DescriptorWriter;
use crate::renderer::subsystems::resource_subsystem::resource_types::texture::{
    ColorTexture, StorageTexture, Texture,
};
use ash::vk;
use crate::renderer::subsystems::resource_subsystem::resource_types::material::Material;

pub(crate) struct ResourceWriter<'a> {
    device: &'a ash::Device,
    command_buffer: &'a vk::CommandBuffer,
    descriptor_writer: DescriptorWriter<'a>,
}

impl<'a> ResourceWriter<'a> {
    pub fn new(device: &'a ash::Device, command_buffer: &'a vk::CommandBuffer) -> Self {
        let descriptor_writer = DescriptorWriter::default();
        Self {
            device,
            command_buffer,
            descriptor_writer,
        }
    }

    pub fn write_render_target_texture(&mut self, texture: &StorageTexture, material: &Material) {
        self.descriptor_writer.write_image(
            0,
            texture.view,
            vk::Sampler::null(),
            vk::ImageLayout::GENERAL,
            vk::DescriptorType::STORAGE_IMAGE,
        );


        self.descriptor_writer.update_set(self.device, material.descriptor_set);
    }
}

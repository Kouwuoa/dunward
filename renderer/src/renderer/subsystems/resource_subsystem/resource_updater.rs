use crate::renderer::subsystems::resource_subsystem::resource_descriptors::descriptor_writer::DescriptorWriter;
use crate::renderer::subsystems::resource_subsystem::resource_types::material::Material;
use crate::renderer::subsystems::resource_subsystem::resource_types::texture::StorageTexture;
use ash::vk;

pub(crate) struct ResourceUpdater<'a> {
    device: &'a ash::Device,
    command_buffer: &'a vk::CommandBuffer,
    updates: Vec<ResourceUpdate<'a>>,
}

impl<'a> ResourceUpdater<'a> {
    pub fn new(device: &'a ash::Device, command_buffer: &'a vk::CommandBuffer) -> Self {
        Self {
            device,
            command_buffer,
            updates: Vec::new(),
        }
    }

    pub fn enqueue_update<F>(&mut self, build_update: F, material: &Material)
    where
        F: FnOnce(&mut ResourceUpdateBuilder),
    {
        let mut builder = ResourceUpdateBuilder::new(material);
        build_update(&mut builder);
        self.updates.push(builder.build());
    }

    pub fn execute_updates(&mut self) {
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
    pub fn new(material: &Material) -> Self {
        let descriptor_writer = DescriptorWriter::default();
        Self {
            descriptor_writer,
            descriptor_set: material.descriptor_set,
        }
    }
    pub fn set_render_target_texture(&mut self, texture: &StorageTexture) {
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

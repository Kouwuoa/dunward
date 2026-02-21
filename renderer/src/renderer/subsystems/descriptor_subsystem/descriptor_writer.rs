use ash::vk;
use crate::renderer::contexts::device_context::DeviceContext;
use crate::renderer::resource_store::megabuffer::Megabuffer;
use crate::renderer::resource_store::texture::Texture;

#[derive(Default)]
pub struct DescriptorWriter<'a> {
    buffer_infos: Vec<(vk::DescriptorBufferInfo, vk::WriteDescriptorSet<'a>)>,
    image_infos: Vec<(vk::DescriptorImageInfo, vk::WriteDescriptorSet<'a>)>,
}

impl<'a> DescriptorWriter<'a> {
    pub fn write_buffer(
        &mut self,
        binding: u32,
        buffer: vk::Buffer,
        size: vk::DeviceSize,
        offset: vk::DeviceSize,
        desc_type: vk::DescriptorType,
    ) {
        let buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(buffer)
            .offset(offset)
            .range(size);
        let buffer_write = vk::WriteDescriptorSet::default()
            .dst_binding(binding)
            .descriptor_count(1)
            .descriptor_type(desc_type);
        self.buffer_infos.push((buffer_info, buffer_write));
    }

    pub fn write_image(
        &mut self,
        binding: u32,
        view: vk::ImageView,
        sampler: vk::Sampler,
        layout: vk::ImageLayout,
        desc_type: vk::DescriptorType,
    ) {
        let image_info = vk::DescriptorImageInfo::default()
            .sampler(sampler)
            .image_view(view)
            .image_layout(layout);
        let image_write = vk::WriteDescriptorSet::default()
            .dst_binding(binding)
            .descriptor_count(1)
            .descriptor_type(desc_type);
        self.image_infos.push((image_info, image_write));
    }

    pub fn clear(&mut self) {
        self.buffer_infos.clear();
        self.image_infos.clear();
    }

    pub fn update_set(&mut self, device: &ash::Device, set: vk::DescriptorSet) {
        let mut writes = Vec::with_capacity(self.buffer_infos.len() + self.image_infos.len());
        writes.extend(self.buffer_infos.iter().map(|(info, write_template)| {
            vk::WriteDescriptorSet {
                dst_set: set,
                p_buffer_info: info,
                ..write_template.clone()
            }
        }));
        writes.extend(self.image_infos.iter().map(|(info, write_template)| {
            vk::WriteDescriptorSet {
                dst_set: set,
                p_image_info: info,
                ..write_template.clone()
            }
        }));
        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }
    }
}

impl DeviceContext {
    pub fn write_texture(&self, tex: &Texture) {
        self.descriptor_writer.write_image

    }
}

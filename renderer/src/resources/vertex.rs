use ash::vk;
use glam::{Vec2, Vec3};
use std::mem::offset_of;
use crate::resources::shader_data::PerVertexData;

#[derive(Debug)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub color: Vec3,
    pub texcoord: Vec2,
}

pub struct VertexInputDescription {
    pub bindings: Vec<vk::VertexInputBindingDescription>,
    pub attributes: Vec<vk::VertexInputAttributeDescription>,
    pub flags: vk::PipelineVertexInputStateCreateFlags,
}

impl Default for VertexInputDescription {
    fn default() -> Self {
        Vertex::get_input_description()
    }
}

impl Vertex {
    pub fn as_shader_data(&self) -> PerVertexData {
        PerVertexData {
            position: self.position,
            texcoord: self.texcoord,
        }
    }

    pub fn get_input_description() -> VertexInputDescription {
        let bindings = vec![vk::VertexInputBindingDescription {
            binding: 0,
            stride: size_of::<Vertex>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }];

        let attributes = vec![
            // Position
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Vertex, position) as u32,
            },
            // Texcoord
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Vertex, texcoord) as u32,
            },
        ];

        let flags = vk::PipelineVertexInputStateCreateFlags::empty();

        VertexInputDescription {
            bindings,
            attributes,
            flags,
        }
    }
}

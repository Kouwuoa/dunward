//! Geometric vertex definitions and Vulkan vertex input attribute bindings.
//!
//! Provides the primary [`Vertex`] structure and helper functions to build
//! [`VertexInputDescription`] for graphics pipeline vertex assembly.

use std::mem::offset_of;

use ash::vk;
use glam::{Vec2, Vec3};

use crate::material::shader_data::PerVertexData;

#[derive(Debug)]
pub(crate) struct Vertex {
    pub(crate) position: Vec3,
    pub(crate) normal: Vec3,
    pub(crate) color: Vec3,
    pub(crate) texcoord: Vec2,
}

pub(crate) struct VertexInputDescription {
    pub(crate) bindings: Vec<vk::VertexInputBindingDescription>,
    pub(crate) attributes: Vec<vk::VertexInputAttributeDescription>,
    pub(crate) flags: vk::PipelineVertexInputStateCreateFlags,
}

impl Default for VertexInputDescription {
    fn default() -> Self {
        Vertex::get_input_description()
    }
}

impl Vertex {
    pub(crate) fn as_shader_data(&self) -> PerVertexData {
        PerVertexData {
            position: self.position,
            texcoord: self.texcoord,
        }
    }

    pub(crate) fn get_input_description() -> VertexInputDescription {
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

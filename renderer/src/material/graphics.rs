//! Graphics material factory builder and pipeline creation.
//!
//! Configures dynamic rendering state, depth-stencil, blending, multisampling,
//! rasterization, vertex input layouts, and pipeline layouts for rasterization passes.

use std::ffi::CString;
use std::sync::{Arc, Mutex};

use ash::vk;
use color_eyre::eyre::{OptionExt, Result, eyre};

use super::MaterialFactory;
use super::shader::GraphicsShader;
use crate::resources::descriptors::DescriptorAllocator;
use crate::scene::vertex::VertexInputDescription;

pub(crate) struct GraphicsMaterialFactoryBuilder<'a> {
    vertex_input_description: VertexInputDescription,
    input_assembly: vk::PipelineInputAssemblyStateCreateInfo<'a>,
    rasterization: vk::PipelineRasterizationStateCreateInfo<'a>,
    color_blend_attachment: vk::PipelineColorBlendAttachmentState,
    multisample: vk::PipelineMultisampleStateCreateInfo<'a>,
    depth_stencil: vk::PipelineDepthStencilStateCreateInfo<'a>,
    color_attachment_format: vk::Format,
    rendering_info: vk::PipelineRenderingCreateInfo<'a>,
    shader: Option<GraphicsShader>,
    pipeline_layout: Option<vk::PipelineLayout>,
    descriptor_set_layout: Option<vk::DescriptorSetLayout>,

    device: Arc<ash::Device>,
    descriptor_allocator: Arc<Mutex<DescriptorAllocator>>,
}

impl<'a> GraphicsMaterialFactoryBuilder<'a> {
    pub fn new(
        device: Arc<ash::Device>,
        descriptor_allocator: Arc<Mutex<DescriptorAllocator>>,
    ) -> Self {
        let vertex_input_description = VertexInputDescription::default();
        let input_assembly = Self::default_input_assembly_info();
        let rasterization = Self::default_rasterization_info();
        let color_blend_attachment = Self::default_color_blend_state();
        let multisample = Self::default_multisample_info();
        let depth_stencil = Self::default_depth_stencil_info();
        let color_attachment_format = vk::Format::UNDEFINED;
        let rendering_info = vk::PipelineRenderingCreateInfo::default();
        let shader = None;
        let pipeline_layout = None;
        let descriptor_set_layout = None;

        Self {
            vertex_input_description,
            input_assembly,
            rasterization,
            color_blend_attachment,
            multisample,
            depth_stencil,
            color_attachment_format,
            rendering_info,
            shader,
            pipeline_layout,
            descriptor_set_layout,

            device,
            descriptor_allocator,
        }
    }

    pub fn with_shader(mut self, shader: GraphicsShader) -> Self {
        let _ = self.shader.replace(shader);
        self
    }

    pub fn with_pipeline_layout(mut self, layout: vk::PipelineLayout) -> Self {
        let _ = self.pipeline_layout.replace(layout);
        self
    }

    pub fn with_descriptor_set_layout(mut self, layout: vk::DescriptorSetLayout) -> Self {
        let _ = self.descriptor_set_layout.replace(layout);
        self
    }

    pub fn with_color_attachment_format(mut self, format: vk::Format) -> Self {
        self.color_attachment_format = format;
        self
    }

    pub fn build(mut self) -> Result<MaterialFactory> {
        let shader = self
            .shader
            .take()
            .ok_or_eyre("No shader provided for GraphicsMaterialBuilder")?;
        let pipeline_layout = self
            .pipeline_layout
            .take()
            .ok_or_eyre("No pipeline layout provided for GraphicsMaterialBuilder")?;
        let descriptor_set_layout = self
            .descriptor_set_layout
            .take()
            .ok_or_eyre("No descriptor set layout provided for GraphicsMaterialBuilder")?;

        let entry_point_name = CString::new("main")?;
        let shader_stages = [
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(shader.vert_mod)
                .name(&entry_point_name),
            vk::PipelineShaderStageCreateInfo::default()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(shader.frag_mod)
                .name(&entry_point_name),
        ];

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&self.vertex_input_description.bindings)
            .vertex_attribute_descriptions(&self.vertex_input_description.attributes)
            .flags(self.vertex_input_description.flags);

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewport_count(1)
            .scissor_count(1);

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_info =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_states);

        let color_blend_attachments = [self.color_blend_attachment];
        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&color_blend_attachments);

        let color_attachment_formats = [self.color_attachment_format];
        let mut rendering_info = self
            .rendering_info
            .color_attachment_formats(&color_attachment_formats);

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&self.input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&self.rasterization)
            .multisample_state(&self.multisample)
            .depth_stencil_state(&self.depth_stencil)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state_info)
            .layout(pipeline_layout)
            .push_next(&mut rendering_info);

        let pipeline = unsafe {
            match self.device.create_graphics_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            ) {
                Ok(pipelines) => Ok(pipelines),
                Err(_) => Err(eyre!("Failed to create graphics pipeline")),
            }
        }?[0];

        Ok(MaterialFactory {
            pipeline,
            pipeline_layout,
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            descriptor_set_layout,
            device: self.device,
            descriptor_allocator: self.descriptor_allocator,
        })
    }

    fn default_input_assembly_info() -> vk::PipelineInputAssemblyStateCreateInfo<'a> {
        vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
    }

    fn default_rasterization_info() -> vk::PipelineRasterizationStateCreateInfo<'a> {
        vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false)
    }

    fn default_color_blend_state() -> vk::PipelineColorBlendAttachmentState {
        vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false)
    }

    fn default_multisample_info() -> vk::PipelineMultisampleStateCreateInfo<'a> {
        vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
    }

    fn default_depth_stencil_info() -> vk::PipelineDepthStencilStateCreateInfo<'a> {
        vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS)
            .depth_bounds_test_enable(false)
            .stencil_test_enable(false)
    }
}

use crate::material::{GraphicsMaterialFactoryBuilder, MaterialFactory, shader::GraphicsShader};

use super::ResourceFactory;

use color_eyre::Result;
use shaderpack::ShaderId;

impl ResourceFactory {
    pub fn create_graphics_material_factory(&self) -> Result<MaterialFactory> {
        let descriptor_set_layout = self.create_graphics_descriptor_set_layout()?;
        let pipeline_layout = self.create_graphics_pipeline_layout(descriptor_set_layout)?;
        let graphics_shader = GraphicsShader::new(
            ShaderId::TriangleVert,
            ShaderId::TriangleFrag,
            self.gpu.raw_logical(),
        )?;
        GraphicsMaterialFactoryBuilder::new(
            self.gpu.raw_logical(),
            self.descriptor_allocator.clone(),
        )
        .with_shader(graphics_shader)
        .with_pipeline_layout(pipeline_layout)
    }

    fn create_graphics_pipeline_layout(&self, descriptor_set_layout: !) -> _ {
        todo!()
    }

    fn create_graphics_descriptor_set_layout(&self) -> _ {
        todo!()
    }
}

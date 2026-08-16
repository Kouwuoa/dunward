//! Shaders, Pipeline States, and Material abstractions.

pub mod material;
pub mod shader;
pub mod shader_data;

pub use material::{
    ComputeMaterialFactoryBuilder, GraphicsMaterialFactoryBuilder, Material, MaterialFactory,
};
pub use shader::{ComputeShader, GraphicsShader};
pub use shader_data::{PerDrawData, PerFrameData, PerMaterialData, PerObjectData, PerVertexData};

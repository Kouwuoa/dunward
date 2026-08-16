//! Scene graph, geometric meshes, vertices, and models.

pub(crate) mod mesh;
pub(crate) mod model;
pub(crate) mod vertex;

pub(crate) use mesh::Mesh;
pub(crate) use model::{FullscreenQuad, Model};
pub(crate) use vertex::{Vertex, VertexInputDescription};

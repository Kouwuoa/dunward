use std::sync::atomic::AtomicU32;
use crate::renderer::resources::vertex::Vertex;

static MESH_ID_COUNTER: AtomicU32 = AtomicU32::new(0);

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Option<Vec<u32>>,
    id: u32,
}

impl Mesh {
    pub fn new(vertices: Vec<Vertex>, indices: Option<Vec<u32>>) -> Self {
        let id = MESH_ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Self {
            vertices,
            indices,
            id,
        }
    }

    pub fn new_triangle() -> Self {
        let vertices = vec![
            Vertex { // Bottom left
                position: [-0.5, -0.5, 0.0].into(),
                normal: [0.0, 0.0, 1.0].into(),
                color: [1.0, 0.0, 0.0].into(),
                texcoord: [0.0, 1.0].into(),
            },
            Vertex { // Bottom right
                position: [0.5, -0.5, 0.0].into(),
                normal: [0.0, 0.0, 1.0].into(),
                color: [0.0, 1.0, 0.0].into(),
                texcoord: [1.0, 1.0].into(),
            },
            Vertex { // Top
                position: [0.0, 0.5, 0.0].into(),
                normal: [0.0, 0.0, 1.0].into(),
                color: [0.0, 0.0, 1.0].into(),
                texcoord: [0.5, 0.0].into(),
            },
        ];

        let indices = vec![0, 1, 2];

        Self::new(vertices, Some(indices))
    }

    pub fn new_quad() -> Self {
        let vertices = vec![
            Vertex { // Top left
                position: [-1.0, 1.0, 0.0].into(),
                normal: [0.0, 0.0, 1.0].into(),
                color: [1.0, 0.0, 0.0].into(),
                texcoord: [0.0, 0.0].into(),
            },
            Vertex { // Bottom left
                position: [-1.0, -1.0, 0.0].into(),
                normal: [0.0, 0.0, 1.0].into(),
                color: [0.0, 1.0, 0.0].into(),
                texcoord: [0.0, 1.0].into(),
            },
            Vertex { // Top right
                position: [1.0, 1.0, 0.0].into(),
                normal: [0.0, 0.0, 1.0].into(),
                color: [0.0, 0.0, 1.0].into(),
                texcoord: [1.0, 0.0].into(),
            },
            Vertex { // Bottom right
                position: [1.0, -1.0, 0.0].into(),
                normal: [0.0, 0.0, 1.0].into(),
                color: [1.0, 1.0, 0.0].into(),
                texcoord: [1.0, 1.0].into(),
            },
        ];

        // Counter-clockwise winding order
        let indices = vec![
            0, 1, 2, // Top left triangle
            2, 1, 3, // Bottom right triangle
        ];

        Self::new(vertices, Some(indices))
    }
}

impl PartialEq for Mesh {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
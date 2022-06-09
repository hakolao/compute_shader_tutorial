use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer},
    device::Device,
};

/// Vertex for textured quads.
#[repr(C)]
#[derive(Default, Debug, Copy, Clone, Zeroable, Pod)]
pub struct TexturedVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
    pub color: [f32; 4],
}
vulkano::impl_vertex!(TexturedVertex, position, tex_coords, color);

/// Textured quad with vertices & indices
#[derive(Default, Debug, Copy, Clone)]
pub struct TexturedQuad {
    pub vertices: [TexturedVertex; 4],
    pub indices: [u32; 6],
}

/// A set of vertices and their indices as cpu accessible buffers
#[derive(Clone)]
pub struct Mesh {
    pub vertices: Arc<CpuAccessibleBuffer<[TexturedVertex]>>,
    pub indices: Arc<CpuAccessibleBuffer<[u32]>>,
}

impl TexturedQuad {
    /// Creates a new textured quad with given width and height at (0.0, 0.0)
    pub fn new(width: f32, height: f32, color: [f32; 4]) -> TexturedQuad {
        TexturedQuad {
            vertices: [
                TexturedVertex {
                    position: [-(width / 2.0), -(height / 2.0)],
                    tex_coords: [0.0, 1.0],
                    color,
                },
                TexturedVertex {
                    position: [-(width / 2.0), height / 2.0],
                    tex_coords: [0.0, 0.0],
                    color,
                },
                TexturedVertex {
                    position: [width / 2.0, height / 2.0],
                    tex_coords: [1.0, 0.0],
                    color,
                },
                TexturedVertex {
                    position: [width / 2.0, -(height / 2.0)],
                    tex_coords: [1.0, 1.0],
                    color,
                },
            ],
            indices: [0, 2, 1, 0, 3, 2],
        }
    }

    /// Converts Quad data to a mesh that can be used in drawing
    pub fn to_mesh(self, device: Arc<Device>) -> Mesh {
        Mesh {
            vertices: CpuAccessibleBuffer::<[TexturedVertex]>::from_iter(
                device.clone(),
                BufferUsage::vertex_buffer(),
                false,
                self.vertices.into_iter(),
            )
            .unwrap(),
            indices: CpuAccessibleBuffer::<[u32]>::from_iter(
                device.clone(),
                BufferUsage::index_buffer(),
                false,
                self.indices.into_iter(),
            )
            .unwrap(),
        }
    }
}

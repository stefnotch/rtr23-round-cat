use super::Vertex;
use crate::vulkan::buffer::Buffer;

pub struct Mesh {
    pub index_buffer: Buffer<u32>,
    pub vertex_buffer: Buffer<Vertex>,
    pub num_indices: u32,
    pub num_vertices: u32,
}

use crate::buffer::Buffer;

use super::Vertex;

pub struct Mesh {
    pub index_buffer: Buffer<u32>,
    pub vertex_buffer: Buffer<Vertex>,
}

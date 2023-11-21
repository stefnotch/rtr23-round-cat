use asset_client::asset_common::gpu::Vertex;

use crate::buffer::Buffer;

pub struct Mesh {
    pub index_buffer: Buffer<u32>,
    pub vertex_buffer: Buffer<Vertex>,
    pub num_indices: u32,
}

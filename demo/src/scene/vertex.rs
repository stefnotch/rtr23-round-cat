use ash::vk;
use asset_client::asset_common::gpu::Vertex;

use crate::offset_of;

pub trait VkVertexDescription {
    fn binding_descriptions() -> [vk::VertexInputBindingDescription; 1];
    fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 4];
}

impl VkVertexDescription for Vertex {
    fn binding_descriptions() -> [vk::VertexInputBindingDescription; 1] {
        [vk::VertexInputBindingDescription {
            binding: 0,
            stride: std::mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }]
    }

    fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 4] {
        [
            vk::VertexInputAttributeDescription {
                location: 0,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Self, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 1,
                binding: 0,
                format: vk::Format::R32G32B32_SFLOAT,
                offset: offset_of!(Self, normal) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 2,
                binding: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: offset_of!(Self, uv) as u32,
            },
            vk::VertexInputAttributeDescription {
                location: 3,
                binding: 0,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: offset_of!(Self, tangent) as u32,
            },
        ]
    }
}

use ultraviolet::Vec3;

use crate::render::shader_types;
use crate::vulkan::buffer::Buffer;
use crate::vulkan::descriptor_set::DescriptorSet;

use super::Texture;

pub struct Material {
    pub base_color: Vec3,
    pub base_color_texture: Texture,
    pub normal_texture: Texture,
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub metallic_roughness_texture: Texture,
    pub emissivity: Vec3,

    pub descriptor_set: DescriptorSet,
    pub descriptor_set_buffer: Buffer<shader_types::Std140Material>,
}

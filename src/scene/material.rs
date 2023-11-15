use ultraviolet::Vec3;

use crate::{buffer::Buffer, descriptor_set::DescriptorSet, render::shader_types};

use super::Texture;

pub struct Material {
    pub base_color: Vec3,
    pub base_color_texture: Texture,
    pub normal_texture: Texture,
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub emissivity: Vec3,

    pub descriptor_set: DescriptorSet,
    pub descriptor_set_buffer: Buffer<shader_types::Std140Material>,
}

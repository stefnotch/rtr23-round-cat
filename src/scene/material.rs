use ultraviolet::Vec3;

use super::Texture;

pub struct Material {
    pub base_color: Vec3,
    pub base_color_texture: Texture,
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub emissivity: Vec3,
}

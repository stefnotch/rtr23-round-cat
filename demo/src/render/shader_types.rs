use crevice::std140::AsStd140;
use ultraviolet::{Mat4, Vec3};

#[derive(AsStd140)]
pub struct Entity {
    pub model: Mat4,
    pub normal_matrix: Mat4,
}

#[derive(AsStd140)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
}

#[derive(AsStd140)]
pub struct Scene {
    pub directional_light: DirectionalLight,
}

#[derive(AsStd140)]
pub struct Material {
    pub base_color: Vec3,
    pub emissivity: Vec3,
    pub roughness: f32,
    pub metallic: f32,
}

#[derive(AsStd140)]
pub struct Camera {
    pub view: Mat4,
    pub proj: Mat4,
}

use ultraviolet::Vec3;

use super::{texture::LoadedTexture, Asset, AssetId};

pub struct LoadedMaterial {
    pub id: AssetId,
    pub base_color: Vec3,
    pub base_color_texture: Option<LoadedTexture>,
    pub normal_texture: Option<LoadedTexture>,
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub metallic_roughness_texture: Option<LoadedTexture>,
    pub emissivity: Vec3,
}

impl LoadedMaterial {
    pub fn missing_material(id: AssetId) -> Self {
        Self {
            id,
            base_color: Vec3::new(0.8, 0.8, 0.0),
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            roughness_factor: 0.0,
            metallic_factor: 0.0,
            emissivity: Vec3::zero(),
        }
    }
}

impl Asset for LoadedMaterial {
    fn id(&self) -> AssetId {
        self.id
    }
}

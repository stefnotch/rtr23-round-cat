use serde::{Deserialize, Serialize};
use ultraviolet::Vec3;

use super::{texture::LoadedTexture, GltfAsset, GltfAssetId, LoadedScene};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoadedMaterial {
    pub id: GltfAssetId,
    pub base_color: Vec3,
    pub base_color_texture: Option<LoadedTexture>,
    pub normal_texture: Option<LoadedTexture>,
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub metallic_roughness_texture: Option<LoadedTexture>,
    pub emissivity: Vec3,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoadedMaterialRef(GltfAssetId);
impl LoadedMaterialRef {
    pub fn new(id: GltfAssetId) -> Self {
        Self(id)
    }

    pub fn get<'a>(&'a self, scene: &'a LoadedScene) -> Option<&'a LoadedMaterial> {
        scene.materials.get(&self)
    }
}

impl LoadedMaterial {
    pub fn missing_material(id: GltfAssetId) -> Self {
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

impl GltfAsset for LoadedMaterial {
    fn id(&self) -> GltfAssetId {
        self.id
    }
}

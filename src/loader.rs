mod asset;
mod material;
mod mesh;
mod model;
mod scene;
mod scene_loader;

pub use asset::*;
pub use material::*;
pub use mesh::*;
pub use model::*;
pub use scene::*;
use ultraviolet::{Rotor3, Vec3};

use crate::transform::Transform;

pub struct AssetLoader {
    pub materials: Assets<LoadedMaterial>,
    pub meshes: Assets<LoadedMesh>,
    pub id_generator: AssetIdGenerator,
}

impl AssetLoader {
    pub fn new() -> Self {
        Self {
            materials: Assets::new(),
            meshes: Assets::new(),
            id_generator: AssetIdGenerator::new(),
        }
    }
}

impl From<gltf::scene::Transform> for Transform {
    fn from(transform: gltf::scene::Transform) -> Self {
        let (translation, rotation, scale) = transform.decomposed();
        Self {
            position: Vec3::from(translation),
            orientation: Rotor3::from_quaternion_array(rotation),
            scale: Vec3::from(scale),
        }
    }
}

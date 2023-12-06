mod material;
mod mesh;
mod texture;
mod vertex;

pub use material::*;
pub use mesh::*;
pub use texture::*;
pub use vertex::*;

use crate::{transform::Transform, vulkan::acceleration_structure::AccelerationStructure};
use std::sync::Arc;

pub struct Scene {
    pub models: Vec<Model>,
    // pub raytracing_scene: RaytracingScene,
}

pub struct Model {
    pub transform: Transform,
    pub primitives: Vec<Primitive>,
}

pub struct Primitive {
    pub material: Arc<Material>,
    pub mesh: Arc<Mesh>,
    pub raytracing_geometry: Arc<RaytracingGeometry>,
}

pub struct RaytracingGeometry {
    pub blas: AccelerationStructure,
}

pub struct RaytracingScene {
    pub tlas: AccelerationStructure,
}

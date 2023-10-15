mod material;
mod mesh;
mod texture;
mod vertex;

pub use material::*;
pub use mesh::*;
pub use vertex::*;

use crate::transform::Transform;
use std::sync::Arc;

pub struct Scene {
    pub models: Vec<Model>,
}

pub struct Model {
    pub transform: Transform,
    pub primitives: Vec<Primitive>,
}

pub struct Primitive {
    pub material: Arc<Material>,
    pub mesh: Arc<Mesh>,
}

use std::sync::Arc;

use crate::transform::Transform;

use super::{LoadedMaterial, LoadedMesh};

pub struct LoadedModel {
    pub transform: Transform,
    pub primitives: Vec<LoadedPrimitive>,
}

pub struct LoadedPrimitive {
    pub material: Arc<LoadedMaterial>,
    pub mesh: Arc<LoadedMesh>,
}

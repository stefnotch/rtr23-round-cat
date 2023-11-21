use serde::{Deserialize, Serialize};

use crate::transform::Transform;

use super::{LoadedMaterialRef, LoadedMeshRef};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoadedModel {
    pub transform: Transform,
    pub primitives: Vec<LoadedPrimitive>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoadedPrimitive {
    pub material: LoadedMaterialRef,
    pub mesh: LoadedMeshRef,
}

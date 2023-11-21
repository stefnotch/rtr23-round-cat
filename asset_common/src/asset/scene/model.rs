use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::transform::Transform;

use super::{LoadedMaterial, LoadedMesh};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoadedModel {
    pub transform: Transform,
    pub primitives: Vec<LoadedPrimitive>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoadedPrimitive {
    // TODO: How does serde handle Arcs?
    pub material: Arc<LoadedMaterial>,
    pub mesh: Arc<LoadedMesh>,
}

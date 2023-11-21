mod gltf_asset;
mod material;
mod mesh;
mod model;
mod texture;

pub use gltf_asset::*;
pub use material::*;
pub use mesh::*;
pub use model::*;
pub use texture::*;

use std::{borrow::Cow, error::Error};

use serde::{Deserialize, Serialize};

use crate::{AssetData, AssetTypeId};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoadedScene {
    pub models: Vec<LoadedModel>,
}

impl LoadedScene {
    pub fn new() -> Self {
        Self { models: Vec::new() }
    }
}

impl AssetData for LoadedScene {
    fn id() -> AssetTypeId
    where
        Self: Sized,
    {
        "scene"
    }

    fn to_bytes(&self) -> Result<Cow<[u8]>, impl Error + 'static> {
        bincode::serialize(self).map(|v| Cow::Owned(v))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, impl Error + 'static> {
        bincode::deserialize(bytes)
    }
}

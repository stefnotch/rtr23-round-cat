use std::{collections::HashMap, sync::Arc};

pub trait GltfAsset {
    fn id(&self) -> GltfAssetId;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct GltfAssetId(u32);
impl GltfAssetId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

pub struct AssetIdGenerator {
    next_id: u32,
}

impl AssetIdGenerator {
    pub fn new() -> Self {
        Self { next_id: 0 }
    }

    pub fn next(&mut self) -> GltfAssetId {
        let id = self.next_id;
        self.next_id += 1;
        GltfAssetId::new(id)
    }
}

pub struct Assets<T: GltfAsset> {
    pub assets: HashMap<GltfAssetId, Arc<T>>,
}

impl<T: GltfAsset> Assets<T> {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }
}

impl<T: GltfAsset> Default for Assets<T> {
    fn default() -> Self {
        Self::new()
    }
}

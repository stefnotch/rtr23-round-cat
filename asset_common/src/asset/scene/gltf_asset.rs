use serde::{Deserialize, Serialize};

pub trait GltfAsset {
    fn id(&self) -> GltfAssetId;
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct GltfAssetId(u32);
impl GltfAssetId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

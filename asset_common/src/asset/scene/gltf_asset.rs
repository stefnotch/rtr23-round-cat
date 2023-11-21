use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct GltfAssetId(u32);
impl GltfAssetId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

use crate::scene::Vertex;

use super::{Asset, AssetId};

pub struct LoadedMesh {
    pub id: AssetId,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Asset for LoadedMesh {
    fn id(&self) -> AssetId {
        self.id
    }
}

use super::{Asset, AssetId};

pub struct LoadedMeshVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tex_coord: [f32; 2],
}

pub struct LoadedMesh {
    pub id: AssetId,
    pub vertices: Vec<LoadedMeshVertex>,
    pub indices: Vec<u32>,
}

impl Asset for LoadedMesh {
    fn id(&self) -> AssetId {
        self.id
    }
}

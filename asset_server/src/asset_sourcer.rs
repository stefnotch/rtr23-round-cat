mod scene_sourcer;
mod shader_sourcer;

pub use scene_sourcer::*;
pub use shader_sourcer::*;

use crate::{
    asset::Asset,
    asset_database::{AssetDatabase, AssetDatabaseMigrated},
    source_files::SourceFileRef,
};

pub trait AssetSourcer<AssetTypes> {
    /// Filters out files that are not relevant for this sourcer.
    /// e.g. A gltf loader would want to read .gltf, .glb and image files.
    fn might_read(&self, path: &SourceFileRef) -> bool;

    fn create(
        &self,
        create_info: CreateAssetInfo,
        asset_database: &AssetDatabase<AssetDatabaseMigrated>,
    ) -> Vec<AssetTypes>;
}

pub struct CreateAssetInfo {
    pub file_ref: SourceFileRef,
    pub asset_name_base: Vec<String>,
}
impl CreateAssetInfo {
    pub fn from_source_file(file_ref: SourceFileRef) -> Self {
        let asset_name_base = file_ref
            .get_path()
            .components()
            .map(|v| v.as_str().into())
            .collect();
        Self {
            file_ref,
            asset_name_base,
        }
    }
}

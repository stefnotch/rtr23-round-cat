use asset_common::AssetRef;

use crate::{
    asset::AssetDependency,
    asset_database::{AssetDatabase, AssetDatabaseMigrated},
    file_change::FileTimestamp,
    source_files::SourceFileRef,
    MyAssetTypes,
};

use super::{Asset, AssetSourcer, CreateAssetInfo};

pub struct SceneSourcer {}

impl SceneSourcer {
    fn is_scene_file(path: &SourceFileRef) -> bool {
        match path.get_path().extension() {
            Some(extension) => extension == "json",
            None => false,
        }
    }
}

impl AssetSourcer<MyAssetTypes> for SceneSourcer {
    fn might_read(&self, path: &SourceFileRef) -> bool {
        Self::is_scene_file(path)
    }

    fn create(
        &self,
        import_request: CreateAssetInfo,
        asset_database: &AssetDatabase<AssetDatabaseMigrated>,
    ) -> Vec<MyAssetTypes> {
        if !Self::is_scene_file(&import_request.file_ref) {
            return vec![];
        }
        let mut imported_asset = Asset::new(
            AssetRef::new(import_request.asset_name_base),
            AssetDependency {
                file: import_request.file_ref.clone(),
                timestamp: FileTimestamp::unknown(),
            },
        );

        imported_asset.try_populate_from_cache_file(
            asset_database
                .get_asset_compilation_file(imported_asset.get_key())
                .ok()
                .flatten(),
        );

        vec![MyAssetTypes::Scene(imported_asset)]
    }
}

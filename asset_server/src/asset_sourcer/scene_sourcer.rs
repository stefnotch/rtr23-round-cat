use asset_common::{scene::LoadedScene, AssetRef};

use crate::{
    asset::AssetDependency, asset_server::AssetInserter, file_change::FileTimestamp,
    source_files::SourceFileRef,
};

use super::{Asset, AssetSourcer, CreateAssetInfo};

pub struct SceneSourcer {}

impl SceneSourcer {
    fn is_scene_file(path: &SourceFileRef) -> bool {
        match path.get_path().extension() {
            Some(extension) => extension == "glb" || extension == "gltf",
            None => false,
        }
    }
}

impl AssetSourcer for SceneSourcer {
    fn might_read(&self, path: &SourceFileRef) -> bool {
        Self::is_scene_file(path)
            || match path.get_path().extension() {
                Some(extension) => extension == "png" || extension == "jpg" || extension == "jpeg",
                None => false,
            }
    }

    fn create_assets(&self, import_request: CreateAssetInfo, asset_server: &mut AssetInserter) {
        if !Self::is_scene_file(&import_request.file_ref) {
            return;
        }
        let mut imported_asset = Asset::<LoadedScene>::new(
            AssetRef::new(import_request.asset_name_base),
            AssetDependency {
                file: import_request.file_ref.clone(),
                timestamp: FileTimestamp::unknown(),
            },
        );

        imported_asset.try_populate_from_cache_file(
            asset_server
                .asset_database
                .get_asset_compilation_file(imported_asset.get_key())
                .ok()
                .flatten(),
        );

        asset_server.all_assets.add_asset(imported_asset);
    }
}

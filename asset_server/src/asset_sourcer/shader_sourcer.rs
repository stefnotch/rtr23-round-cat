use asset_common::{shader::Shader, AssetRef};

use crate::{
    asset::AssetDependency, file_change::FileTimestamp, source_files::SourceFileRef, AssetInserter,
};

use super::{Asset, AssetSourcer, CreateAssetInfo};

pub struct ShaderSourcer {}

impl ShaderSourcer {
    fn is_shader_file(path: &SourceFileRef) -> bool {
        match path.get_path().extension() {
            Some(extension) => extension == "glsl" || extension == "frag" || extension == "vert",
            None => false,
        }
    }
}

impl AssetSourcer for ShaderSourcer {
    fn might_read(&self, path: &SourceFileRef) -> bool {
        Self::is_shader_file(path)
    }

    fn create_assets(&self, import_request: CreateAssetInfo, asset_server: &mut AssetInserter) {
        if !Self::is_shader_file(&import_request.file_ref) {
            return;
        }
        let mut imported_asset = Asset::<Shader>::new(
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

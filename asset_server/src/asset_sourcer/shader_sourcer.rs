use crate::{
    asset::{AssetDependency, AssetRef, AssetType},
    file_change::FileTimestamp,
    source_files::SourceFileRef,
    MyAssetTypes,
};

use super::{Asset, AssetSourcer, CreateAssetInfo};

pub struct ShaderSourcer {}

impl AssetSourcer<MyAssetTypes> for ShaderSourcer {
    fn can_potentially_handle(&self, path: &SourceFileRef) -> bool {
        match path.get_path().extension() {
            Some(extension) => extension == "glsl" || extension == "frag" || extension == "vert",
            None => false,
        }
    }

    fn create(&self, import_request: CreateAssetInfo) -> Vec<MyAssetTypes> {
        // We simply assume that it's a valid shader.
        // Compilation is done later, on-demand.
        let imported_asset = Asset::new(
            AssetRef {
                name: import_request.asset_name_base,
                asset_type: AssetType::Shader,
            },
            AssetDependency {
                file: import_request.file_ref.clone(),
                timestamp: FileTimestamp::unknown(),
            },
        );

        vec![MyAssetTypes::Shader(imported_asset)]
    }
}

use crate::source_files::SourceFileRef;

use super::{Asset, AssetSourcer, AssetType, CreateAssetInfo};

pub struct ShaderSourcer {}

impl AssetSourcer for ShaderSourcer {
    fn can_potentially_handle(&self, path: &SourceFileRef) -> bool {
        match path.get_path().extension() {
            Some(extension) => extension == "glsl" || extension == "frag" || extension == "vert",
            None => false,
        }
    }

    fn create(&self, import_request: CreateAssetInfo) -> Vec<Asset> {
        // We simply assume that it's a valid shader.
        // Compilation is done later, on-demand.
        let imported_asset = Asset::new(
            import_request.asset_ref_base,
            AssetType::Shader,
            import_request.file_ref.clone(),
        );

        vec![imported_asset]
    }
}

use std::sync::Arc;

use asset_client::{
    asset_common::{shader::Shader, AssetHandle, Entrypoint},
    AssetClient,
};

pub struct MainAssets {
    pub assets: AssetCollectionFile,
    pub asset_client: Arc<AssetClient>,
}

impl MainAssets {
    pub fn load(asset_client: Arc<AssetClient>) -> Self {
        let data = asset_client.load(&Entrypoint::new().main_assets);
        let assets = serde_json::from_slice(&data.data).unwrap();
        Self {
            assets,
            asset_client,
        }
    }
}

#[derive(serde::Deserialize)]
pub struct AssetCollectionFile {
    pub gbuffer_frag_shader: AssetHandle<Shader>,
    pub gbuffer_vert_shader: AssetHandle<Shader>,
    pub light_frag_shader: AssetHandle<Shader>,
    pub light_vert_shader: AssetHandle<Shader>,
}

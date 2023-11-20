use std::sync::Arc;

use asset_client::{
    asset_common::{shader::Shader, AssetHandle, Entrypoint},
    AssetClient,
};

pub struct MainScene {
    pub scene: SceneFile,
    pub asset_client: Arc<AssetClient>,
}

impl MainScene {
    pub fn load(asset_client: Arc<AssetClient>) -> Self {
        let scene_bytes = asset_client.load(&Entrypoint::new().main_scene);
        let scene = serde_json::from_slice(&scene_bytes.data).unwrap();
        Self {
            scene,
            asset_client,
        }
    }
}

#[derive(serde::Deserialize)]
pub struct SceneFile {
    pub gbuffer_frag_shader: AssetHandle<Shader>,
    pub gbuffer_vert_shader: AssetHandle<Shader>,
    pub light_frag_shader: AssetHandle<Shader>,
    pub light_vert_shader: AssetHandle<Shader>,
}

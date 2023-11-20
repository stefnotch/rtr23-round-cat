use std::sync::Arc;

use asset_client::{
    asset_common::{scene::Scene, shader::Shader, AssetRef},
    AssetClient, AssetHandle,
};

pub struct MainScene {
    pub scene: SceneFile,
    pub asset_client: Arc<AssetClient>,
}

impl MainScene {
    pub fn load(asset_client: Arc<AssetClient>) -> Self {
        let scene_handle =
            AssetHandle::<Scene>::new_unchecked(AssetRef::new(vec!["scene.json".into()]));

        let scene_bytes = scene_handle.load(&asset_client);

        Self {
            scene: serde_json::from_slice(&scene_bytes.data).unwrap(),
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

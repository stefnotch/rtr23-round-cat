mod asset;
pub mod ipc;

use asset::scene::Scene;
pub use asset::*;

pub struct Entrypoint {
    pub main_scene: AssetHandle<Scene>,
}

impl Entrypoint {
    pub fn new() -> Self {
        Self {
            main_scene: AssetHandle::<Scene>::new_unchecked(AssetRef::new(vec![
                "scene.json".into()
            ])),
        }
    }
}

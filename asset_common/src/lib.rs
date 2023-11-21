mod asset;
pub mod ipc;

use asset::asset_collection::AssetCollection;
pub use asset::*;

pub struct Entrypoint {
    pub main_assets: AssetHandle<AssetCollection>,
}

impl Entrypoint {
    pub fn new() -> Self {
        Self {
            main_assets: AssetHandle::<AssetCollection>::new_unchecked(AssetRef::new(vec![
                "asset_collection.json".into(),
            ])),
        }
    }
}

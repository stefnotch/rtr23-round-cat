use std::path::PathBuf;

pub struct AssetsConfig {
    pub version: u64,
    pub source: PathBuf,
    pub target: PathBuf,
}

impl AssetsConfig {
    pub fn get_asset_cache_db_path(&self) -> PathBuf {
        let mut path = self.target.clone();
        path.push(&"asset_cache.bin");
        path
    }
}

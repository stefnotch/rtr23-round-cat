use std::path::PathBuf;

pub struct AssetsConfig {
    pub source: PathBuf,
    pub target: PathBuf,
}

impl AssetsConfig {
    pub fn get_source_file_path(&self) -> PathBuf {
        let mut path = self.target.clone();
        path.push(&"source_file_cache.bin");
        path
    }
}

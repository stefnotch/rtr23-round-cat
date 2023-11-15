mod asset_processor;
mod assets_config;
mod read_startup;
use std::{collections::HashMap, fs, path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};

use crate::{
    asset_processor::AssetProcessor, assets_config::AssetsConfig, read_startup::read_startup,
};

#[derive(Serialize, Deserialize)]
pub struct SourceFiles {
    pub version: u64,
    pub files: HashMap<PathBuf, SourceFileData>,
}
impl SourceFiles {
    pub fn new() -> Self {
        Self {
            version: 0,
            files: Default::default(),
        }
    }
}
impl Default for SourceFiles {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize)]
pub struct SourceFileData {
    pub last_changed: Option<SystemTime>,
    pub is_dirty: bool,
}

#[tokio::main]
async fn main() {
    let config = AssetsConfig {
        source: "assets".into(),
        target: "target-assets".into(),
    };

    let asset_processors: Vec<Box<dyn AssetProcessor>> = vec![];

    let old_source_files = fs::read(config.get_source_file_path())
        .ok()
        .and_then(|v| bincode::deserialize(&v).ok())
        .unwrap_or_default();
    let source_files = read_startup(&config, &old_source_files, &asset_processors);

    println!("Hello, world!");
}

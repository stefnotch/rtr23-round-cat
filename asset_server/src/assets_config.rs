use std::path::{Path, PathBuf};

use relative_path::PathExt;

use crate::source_files::SourceFileRef;

pub struct AssetsConfig {
    pub version: u64,
    pub source: PathBuf,
    pub target: PathBuf,
}

impl AssetsConfig {
    pub fn get_asset_cache_db_path(&self) -> PathBuf {
        self.target.join("asset_cache.redb")
    }

    pub fn get_asset_schema_path(&self) -> PathBuf {
        self.target.join("schema.json")
    }
}

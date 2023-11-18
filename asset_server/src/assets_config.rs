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

    pub fn get_source_file_ref(&self, path: &Path) -> SourceFileRef {
        SourceFileRef::new(path.relative_to(&self.source).unwrap_or_else(|error| {
            panic!(
                "Failed to get relative path for {:?} with base {:?}, because of {:?}",
                path, &self.source, error
            )
        }))
    }
}

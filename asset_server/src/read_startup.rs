use relative_path::{PathExt, RelativePathBuf};
use walkdir::WalkDir;

use crate::{
    asset_sourcer::AssetSourcer,
    assets_config::AssetsConfig,
    source_files::{SourceFileData, SourceFileRef, SourceFiles},
};

pub fn read_startup(
    config: &AssetsConfig,
    asset_sourcers: &[Box<dyn AssetSourcer>],
) -> SourceFiles {
    let mut source_files = SourceFiles::new();
    for entry in WalkDir::new(&config.source)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let relative_path = entry
            .path()
            .relative_to(config.get_asset_cache_db_path())
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to get relative path for {:?} with base {:?}",
                    entry.path(),
                    config.get_asset_cache_db_path()
                )
            });
        let path = SourceFileRef::new(relative_path);

        if !asset_sourcers
            .iter()
            .any(|v| v.can_potentially_handle(&path))
        {
            continue;
        }

        let last_changed = match entry.metadata() {
            Ok(v) => v.modified().ok(),
            Err(e) => {
                log::warn!("Failed to get metadata for {:?}: {}", entry.path(), e);
                continue;
            }
        };
        source_files
            .files
            .insert(path, SourceFileData { last_changed });
    }

    source_files
}

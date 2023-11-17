use relative_path::{PathExt, RelativePathBuf};
use walkdir::WalkDir;

use crate::{
    asset_sourcer::AssetSourcer,
    assets_config::AssetsConfig,
    source_files::{SourceFileData, SourceFileRef, SourceFiles},
};

pub fn read_startup(
    config: &AssetsConfig,
    old_source_files: &SourceFiles,
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
            .relative_to(config.get_source_file_path())
            .unwrap_or_else(|_| {
                panic!(
                    "Failed to get relative path for {:?} with base {:?}",
                    entry.path(),
                    config.get_source_file_path()
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
            // Maybe log this?
            Err(_) => continue,
        };
        let is_dirty = match (last_changed, old_source_files.files.get(&path)) {
            (
                Some(ref last_changed),
                Some(SourceFileData {
                    last_changed: Some(old_last_changed),
                    is_dirty: false,
                }),
            ) => last_changed <= old_last_changed,
            _ => true,
        };
        source_files.files.insert(
            path,
            SourceFileData {
                last_changed,
                is_dirty,
            },
        );
    }

    source_files
}

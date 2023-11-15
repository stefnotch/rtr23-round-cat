use walkdir::WalkDir;

use crate::{
    asset_processor::AssetProcessor, assets_config::AssetsConfig, SourceFileData, SourceFiles,
};

pub fn read_startup(
    config: &AssetsConfig,
    old_source_files: &SourceFiles,
    asset_processors: &[Box<dyn AssetProcessor>],
) -> SourceFiles {
    let mut source_files = SourceFiles::new();
    for entry in WalkDir::new(&config.source)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        if !asset_processors
            .iter()
            .any(|v| v.can_potentially_handle(entry.path()))
        {
            continue;
        }

        let path = entry.path().to_path_buf();
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

use std::collections::HashMap;

use walkdir::WalkDir;

use crate::{
    asset_sourcer::AssetSourcer,
    assets_config::AssetsConfig,
    file_change::FileTimestamp,
    source_files::{SourceFileData, SourceFiles},
};

impl SourceFiles {
    pub fn read_startup(
        config: &AssetsConfig,
        asset_sourcers: &[Box<dyn AssetSourcer>],
    ) -> SourceFiles {
        let mut source_files = HashMap::new();
        for entry in WalkDir::new(&config.source)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = config.get_source_file_ref(entry.path());
            if !asset_sourcers
                .iter()
                .any(|v| v.can_potentially_handle(&path))
            {
                continue;
            }

            let timestamp = match entry.metadata() {
                Ok(v) => {
                    FileTimestamp::new(v.modified().expect("Failed to get modified timestamp"))
                }
                Err(e) => {
                    log::warn!("Failed to get metadata for {:?}: {}", entry.path(), e);
                    continue;
                }
            };
            source_files.insert(path, SourceFileData { timestamp });
        }

        SourceFiles::new(source_files)
    }
}

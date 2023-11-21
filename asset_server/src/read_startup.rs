use walkdir::WalkDir;

use crate::{
    asset_sourcer::{AssetSourcer, CreateAssetInfo},
    file_change::FileTimestamp,
    source_files::{SourceFileRef, SourceFileUpdate, SourceFiles},
    AssetInserter, MyAssetServer,
};

fn read_startup(source_files: &mut SourceFiles, asset_sourcers: &[Box<dyn AssetSourcer>]) {
    let base_path = { source_files.take_snapshot().base_path().to_path_buf() };

    for entry in WalkDir::new(&base_path).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let path = SourceFileRef::new(entry.path(), &base_path);
        if !asset_sourcers.iter().any(|v| v.might_read(&path)) {
            continue;
        }

        let timestamp = match entry.metadata() {
            Ok(v) => FileTimestamp::new(v.modified().expect("Failed to get modified timestamp")),
            Err(e) => {
                log::warn!("Failed to get metadata for {:?}: {}", entry.path(), e);
                continue;
            }
        };
        source_files.update(SourceFileUpdate::Insert(path, timestamp));
    }
}

impl MyAssetServer {
    pub fn load_startup(&mut self) {
        read_startup(&mut self.source_files, &self.asset_sourcers);
        loop {
            let source_ref = match self.source_files.try_take_changed() {
                Some(source_ref) => source_ref,
                None => break,
            };

            let mut asset_inserter = AssetInserter {
                source_files: &self.source_files,
                asset_database: &self.asset_database,
                all_assets: &mut self.all_assets,
            };
            for asset_sourcer in self.asset_sourcers.iter() {
                if !asset_sourcer.might_read(&source_ref) {
                    continue;
                }
                asset_sourcer.create_assets(
                    CreateAssetInfo::from_source_file(source_ref.clone()),
                    &mut asset_inserter,
                );
            }
        }
    }
}

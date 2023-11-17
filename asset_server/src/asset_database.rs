use std::{collections::HashMap, io};

use crate::{
    asset_file::AssetFileInfo,
    asset_sourcer::{Asset, AssetRef},
    source_files::{SourceFileRef, SourceFiles},
};
use sled::{Config, Db};

pub struct AssetDatabase<State> {
    db: Db,
    _state: State,
}

pub struct AssetDatabaseNew;
pub struct AssetDatabaseMigrated;

impl AssetDatabase<AssetDatabaseNew> {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            _state: AssetDatabaseNew,
        }
    }

    pub fn needs_migration(&self, version: u64) -> bool {
        // Poor person's try block, see https://github.com/rust-lang/rust/issues/31436#issuecomment-1736412533
        (|| {
            let metadata = &self.open_metadata_tree().ok()?;
            let old_version = metadata.get(Self::metadata_version_key()).ok().flatten()?;
            let old_version = (&*old_version).try_into().ok()?;
            Some(u64::from_le_bytes(old_version) < version)
        })()
        .unwrap_or(true)
    }

    pub fn finished_migration(self) -> AssetDatabase<AssetDatabaseMigrated> {
        AssetDatabase {
            db: self.db,
            _state: AssetDatabaseMigrated,
        }
    }
}

impl<State> AssetDatabase<State> {
    fn open_metadata_tree(&self) -> sled::Result<sled::Tree> {
        self.db.open_tree(b"metadata")
    }
    const fn metadata_version_key() -> &'static [u8] {
        b"version"
    }
}

impl AssetDatabase<AssetDatabaseMigrated> {
    fn open_asset_file_info_tree(&self) -> sled::Result<sled::Tree> {
        self.db.open_tree(b"asset_file_info")
    }

    pub fn get_asset_file_info(&self, key: &AssetRef) -> io::Result<Option<AssetFileInfo>> {
        let asset_file_info_tree = self.open_asset_file_info_tree()?;
        let binary_key = bincode::serialize(key).unwrap();
        let asset_file_info = match asset_file_info_tree.get(&binary_key)? {
            Some(data) => bincode::deserialize::<Option<AssetFileInfo>>(&data),
            None => return Ok(None),
        };

        match asset_file_info {
            Ok(asset_file_info) => Ok(asset_file_info),
            Err(err) => {
                log::error!("Failed to deserialize asset file info: {:?}", err);
                Ok(None)
            }
        }
    }
}

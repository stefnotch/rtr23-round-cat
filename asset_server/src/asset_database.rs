

use redb::{Database, ReadableTable, TableDefinition};



use crate::{
    asset::{AssetRef},
    asset_cache::AssetCompilationFile,
};

pub struct AssetDatabase<State> {
    db: Database,
    _state: State,
}

pub struct AssetDatabaseNew;
pub struct AssetDatabaseMigrated;

impl AssetDatabase<AssetDatabaseNew> {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            _state: AssetDatabaseNew,
        }
    }

    pub fn needs_migration(&self, version: u64) -> bool {
        // Poor person's try block, see https://github.com/rust-lang/rust/issues/31436#issuecomment-1736412533
        (|| {
            let transaction = self.db.begin_read().ok()?;
            let metadata = transaction.open_table(METADATA_TABLE).ok()?;
            let old_version = metadata.get(Self::metadata_version_key()).ok().flatten()?;
            let old_version = old_version.value().try_into().ok()?;
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

const METADATA_TABLE: TableDefinition<&str, Vec<u8>> = TableDefinition::new("metadata");
impl<State> AssetDatabase<State> {
    const fn metadata_version_key() -> &'static str {
        "version"
    }
}

const ASSET_FILE_INFO_TABLE: TableDefinition<&[u8], Vec<u8>> =
    TableDefinition::new("asset_file_info");
impl AssetDatabase<AssetDatabaseMigrated> {
    pub fn get_asset_compilation_file(
        &self,
        key: &AssetRef,
    ) -> anyhow::Result<Option<AssetCompilationFile>> {
        let transaction = self.db.begin_read()?;

        let asset_file_info_tree = transaction.open_table(ASSET_FILE_INFO_TABLE)?;
        let binary_key = bincode::serialize(key).unwrap();
        let asset_file_info = match asset_file_info_tree.get(&binary_key[..])? {
            Some(data) => bincode::deserialize::<Option<AssetCompilationFile>>(&data.value()),
            None => return Ok(None),
        };

        match asset_file_info {
            Ok(asset_file_info) => Ok(asset_file_info),
            Err(err) => {
                log::error!("Failed to deserialize asset file info: {:?}", err);
                Err(err)?
            }
        }
    }

    pub fn set_asset_compilation_file(
        &self,
        key: &AssetRef,
        compilation_file: AssetCompilationFile,
    ) -> anyhow::Result<()> {
        let binary_key = bincode::serialize(key)?;
        let binary_value = bincode::serialize(&compilation_file)?;

        let transaction = self.db.begin_write()?;
        {
            let mut asset_file_info_tree = transaction.open_table(ASSET_FILE_INFO_TABLE)?;
            asset_file_info_tree.insert(&binary_key[..], binary_value)?;
        }
        transaction.commit()?;

        Ok(())
    }
}

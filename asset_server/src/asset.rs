use asset_common::{AssetData, AssetRef};
use std::{collections::HashSet, sync::Arc};

use serde::{Deserialize, Serialize};

use crate::{
    asset_compilation::AssetCompilationFile,
    asset_database::{AssetDatabase, AssetDatabaseMigrated},
    asset_loader::AssetLoader,
    assets_config::AssetsConfig,
    file_change::FileTimestamp,
    source_files::{SourceFileRef, SourceFiles},
};

/// A lazily loaded asset.
#[derive(Clone, Debug)]
pub struct Asset<Data: AssetData> {
    pub key: AssetRef,
    pub main_file: AssetDependency,
    /// Can also reference currently nonexistent files.
    /// Main file is implicitly included.
    pub dependencies: HashSet<AssetDependency>,

    pub data: Option<Arc<Data>>,
}

impl<Data: AssetData> Asset<Data> {
    pub fn new(key: AssetRef, main_file: AssetDependency) -> Self {
        Self {
            key,
            main_file,
            dependencies: HashSet::new(),
            data: None,
        }
    }

    pub fn main_file_ref(&self) -> &SourceFileRef {
        &self.main_file.file
    }

    pub fn get_key(&self) -> &AssetRef {
        &self.key
    }

    pub fn try_populate_from_cache_file(&mut self, asset_cache_file: Option<AssetCompilationFile>) {
        let asset_cache_file = match asset_cache_file {
            Some(asset_cache_file) => asset_cache_file,
            None => return,
        };
        self.main_file.timestamp = asset_cache_file.main_file.timestamp;
        assert!(self.main_file.file == asset_cache_file.main_file.file);
        self.dependencies = asset_cache_file.dependencies;
    }

    pub fn compile_if_outdated(
        &mut self,
        loader: &impl AssetLoader<AssetData = Data>,
        asset_database: &AssetDatabase<AssetDatabaseMigrated>,
        config: &AssetsConfig,
        source_files: &SourceFiles,
    ) -> anyhow::Result<AssetCompilationFile> {
        if let Ok(Some(asset_cache_file)) = asset_database.get_asset_compilation_file(&self.key) {
            if !asset_cache_file.is_outdated(self) {
                // No compilation necessary
                return Ok(asset_cache_file);
            }
        }

        let compile_result = loader.compile_asset(self, config, source_files)?; // Potentially slow
        asset_database.set_asset_compilation_file(&self.key, &compile_result.compilation_file)?;
        self.data = compile_result.data.map(Arc::new);
        Ok(compile_result.compilation_file)
    }

    /// Does the entire "check if outdated", "compile if necessary", "load asset" dance.
    pub fn load(
        &mut self,
        loader: &impl AssetLoader<AssetData = Data>,
        asset_database: &AssetDatabase<AssetDatabaseMigrated>,
        config: &AssetsConfig,
        source_files: &SourceFiles,
    ) -> anyhow::Result<Arc<Data>> {
        let compile_result =
            self.compile_if_outdated(loader, asset_database, config, source_files)?; // Potentially slow

        if let Some(data) = self.data.clone() {
            return Ok(data);
        } else {
            let data = loader
                .load_asset(&compile_result, config, source_files)
                .map(Arc::new)?; // Potentially slow
            self.data = Some(data.clone());
            return Ok(data);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct AssetDependency {
    pub file: SourceFileRef,
    pub timestamp: FileTimestamp,
}
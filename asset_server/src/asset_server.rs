use std::{collections::HashMap, fs, path::PathBuf, sync::Arc};

use asset_common::{AssetData, AssetRef, AssetTypeId};

use crate::{
    asset::Asset,
    asset_database::AssetDatabase,
    asset_database::AssetDatabaseMigrated,
    asset_loader::AssetLoader,
    asset_sourcer::AssetSourcer,
    assets_config::AssetsConfig,
    json_schema::AssetJsonSchema,
    source_files::{SourceFileRef, SourceFiles},
};

pub struct AllAssets {
    all_assets: HashMap<AssetTypeId, Box<dyn AssetsContainer>>,
}
impl AllAssets {
    pub fn new() -> Self {
        Self {
            all_assets: HashMap::new(),
        }
    }

    pub fn with_asset_type<T: AssetData + 'static>(
        mut self,
        loader: impl AssetLoader<AssetData = T> + 'static,
    ) -> Self {
        let assets = Assets::<T>::new(loader);
        self.all_assets.insert(T::id(), Box::new(assets));
        self
    }

    fn get_typed_assets_mut<T: AssetData + 'static>(&mut self) -> &mut Assets<T> {
        self.all_assets
            .get_mut(&T::id())
            .expect("Asset type not registered")
            .as_any_mut()
            .downcast_mut::<Assets<T>>()
            .expect("Asset type mismatch")
    }

    fn get_typed_assets<T: AssetData + 'static>(&self) -> &Assets<T> {
        self.all_assets
            .get(&T::id())
            .expect("Asset type not registered")
            .as_any()
            .downcast_ref::<Assets<T>>()
            .expect("Asset type mismatch")
    }

    pub fn all_asset_keys<'a>(&'a self) -> impl Iterator<Item = &'a AssetRef> {
        self.all_assets
            .values()
            .flat_map(|assets| assets.get_keys())
    }

    pub fn get_asset_mut<'a, T: AssetData + 'static>(
        &'a mut self,
        asset_ref: &AssetRef,
    ) -> Option<&'a mut Asset<T>> {
        self.get_typed_assets_mut().assets.get_mut(asset_ref)
    }

    pub fn get_asset<'a, T: AssetData + 'static>(
        &'a self,
        asset_ref: &AssetRef,
    ) -> Option<&'a Asset<T>> {
        self.get_typed_assets().assets.get(asset_ref)
    }

    pub fn load_asset<T: AssetData + 'static>(
        &mut self,
        source_files: &SourceFiles,
        asset_database: &AssetDatabase<AssetDatabaseMigrated>,
        request: AssetRef,
    ) -> anyhow::Result<Arc<T>> {
        let assets = self.get_typed_assets_mut::<T>();
        let loader = &assets.loader;
        let asset = assets
            .assets
            .get_mut(&request)
            .ok_or_else(|| anyhow::format_err!("Asset not found {:?}", request))?;

        let asset_data = asset.load(loader, asset_database, source_files)?;

        Ok(asset_data)
    }

    pub fn add_asset<T: AssetData + 'static>(&mut self, asset: Asset<T>) {
        self.get_typed_assets_mut().add_asset(asset);
    }
}

trait AssetsContainer {
    fn get_keys(&self) -> Box<dyn Iterator<Item = &AssetRef> + '_>;
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

impl<T: AssetData + 'static> AssetsContainer for Assets<T> {
    fn get_keys(&self) -> Box<dyn Iterator<Item = &AssetRef> + '_> {
        Box::new(self.assets.keys())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct Assets<T: AssetData + 'static + ?Sized> {
    pub loader: Box<dyn AssetLoader<AssetData = T>>,
    pub assets: HashMap<AssetRef, Asset<T>>,
    pub asset_dependencies_inverse: HashMap<SourceFileRef, Vec<AssetRef>>,
}
impl<T: AssetData> Assets<T> {
    pub fn new(loader: impl AssetLoader<AssetData = T> + 'static) -> Self {
        Self {
            loader: Box::new(loader),
            assets: HashMap::new(),
            asset_dependencies_inverse: HashMap::new(),
        }
    }

    pub fn add_asset(&mut self, asset: Asset<T>) {
        for dependency in asset.dependencies.iter() {
            self.asset_dependencies_inverse
                .entry(dependency.file.clone())
                .or_default()
                .push(asset.key.clone());
        }
        self.assets.insert(asset.key.clone(), asset);
    }
}

pub struct MyAssetServer {
    pub source_files: SourceFiles,
    pub asset_sourcers: Vec<Box<dyn AssetSourcer>>,
    pub asset_database: AssetDatabase<AssetDatabaseMigrated>,

    // See also typed registry from https://arxiv.org/pdf/2307.07069.pdf
    pub all_assets: AllAssets,
}

pub struct AssetInserter<'a> {
    pub source_files: &'a SourceFiles,
    pub asset_database: &'a AssetDatabase<AssetDatabaseMigrated>,
    pub all_assets: &'a mut AllAssets,
}

impl MyAssetServer {
    pub fn load_asset<T: AssetData + 'static>(
        &mut self,
        request: AssetRef,
    ) -> anyhow::Result<Arc<T>> {
        self.all_assets
            .load_asset(&self.source_files, &self.asset_database, request)
    }

    pub fn write_schema_file(&self) -> anyhow::Result<()> {
        let schema = AssetJsonSchema::create_schema(self.all_assets.all_asset_keys());
        std::fs::write(self.get_asset_schema_path(), schema)?;
        Ok(())
    }

    pub fn get_asset_schema_path(&self) -> PathBuf {
        self.asset_database.get_target_path().join("schema.json")
    }
}

pub fn load_asset_database(
    config: &AssetsConfig,
) -> anyhow::Result<AssetDatabase<AssetDatabaseMigrated>> {
    let database_config = redb::Builder::new();

    let mut asset_database = AssetDatabase::new(
        database_config.create(config.get_asset_cache_db_path())?,
        config.target.clone(),
    );
    if asset_database.needs_migration(config.version) {
        std::mem::drop(asset_database);
        fs::remove_dir_all(&config.target)?;
        fs::create_dir_all(&config.target)?;
        asset_database = AssetDatabase::new(
            database_config.create(config.get_asset_cache_db_path())?,
            config.target.clone(),
        );
    }
    Ok(asset_database.finished_migration())
}

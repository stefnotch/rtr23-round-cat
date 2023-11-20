pub mod asset;
pub mod asset_compilation;
pub mod asset_database;
pub mod asset_loader;
pub mod asset_sourcer;
pub mod assets_config;
pub mod file_change;
pub mod json_schema;
pub mod read_startup;
pub mod source_files;
use std::{collections::HashMap, sync::Arc};

use asset_common::{scene::Scene, shader::Shader, AssetData, AssetRef, AssetTypeId};
use asset_loader::AssetLoader;
use asset_sourcer::AssetSourcer;

use crate::{
    asset::Asset,
    asset_database::AssetDatabase,
    asset_database::AssetDatabaseMigrated,
    assets_config::AssetsConfig,
    json_schema::AssetJsonSchema,
    source_files::{SourceFileRef, SourceFiles},
};

pub enum MyAssetTypes {
    Shader(Asset<Shader>),
    Scene(Asset<Scene>),
    // Model(Asset<ModelLoader>),
}

pub struct AllAssets {
    all_assets: HashMap<AssetTypeId, Box<dyn std::any::Any>>,
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
        self.all_assets
            .insert(T::id(), Box::new(Assets::<T>::new(loader)));
        self
    }

    fn get_typed_assets_mut<T: AssetData + 'static>(&mut self) -> &mut Assets<T> {
        self.all_assets
            .get_mut(&T::id())
            .expect("Asset type not registered")
            .downcast_mut::<Assets<T>>()
            .expect("Asset type mismatch")
    }

    fn get_typed_assets<T: AssetData + 'static>(&self) -> &Assets<T> {
        self.all_assets
            .get(&T::id())
            .expect("Asset type not registered")
            .downcast_ref::<Assets<T>>()
            .expect("Asset type mismatch")
    }

    pub fn all_asset_keys<'a>(&'a self) -> impl Iterator<Item = &'a AssetRef> {
        self.all_assets.values().flat_map(|assets| {
            assets
                .downcast_ref::<Assets<dyn AssetData>>()
                .unwrap()
                .assets
                .keys()
        })
    }

    pub fn get_asset_mut<'a, T: AssetData + 'static>(
        &'a mut self,
        asset_ref: &AssetRef,
    ) -> Option<&'a mut Asset<T>> {
        let assets = self
            .all_assets
            .get_mut(&T::id())
            .expect("Asset type not registered")
            .downcast_mut::<Assets<T>>()
            .expect("Asset type mismatch");

        assets.assets.get_mut(asset_ref)
    }

    pub fn get_asset<'a, T: AssetData + 'static>(
        &'a self,
        asset_ref: &AssetRef,
    ) -> Option<&'a Asset<T>> {
        let assets = self
            .all_assets
            .get(&T::id())
            .expect("Asset type not registered")
            .downcast_ref::<Assets<T>>()
            .expect("Asset type mismatch");

        assets.assets.get(asset_ref)
    }

    pub fn load_asset<T: AssetData + 'static>(
        &mut self,
        config: &AssetsConfig,
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

        let asset_data = asset.load(loader, asset_database, config, source_files)?;

        Ok(asset_data)
    }

    pub fn add_asset<T: AssetData + 'static>(&mut self, asset: Asset<T>) {
        let assets = self
            .all_assets
            .get_mut(&T::id())
            .expect("Asset type not registered")
            .downcast_mut::<Assets<T>>()
            .expect("Asset type mismatch");

        assets.add_asset(asset);
    }
}

pub struct Assets<T: AssetData + ?Sized> {
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
    pub config: AssetsConfig,
    pub source_files: SourceFiles,
    pub asset_sourcers: Vec<Box<dyn AssetSourcer<MyAssetTypes>>>,
    pub asset_database: AssetDatabase<AssetDatabaseMigrated>,

    // See also typed registry from https://arxiv.org/pdf/2307.07069.pdf
    pub all_assets: AllAssets,
}

impl MyAssetServer {
    pub fn load_asset<T: AssetData + 'static>(
        &mut self,
        request: AssetRef,
    ) -> anyhow::Result<Arc<T>> {
        self.all_assets.load_asset(
            &self.config,
            &self.source_files,
            &self.asset_database,
            request,
        )
    }

    pub fn write_schema_file(&self) -> anyhow::Result<()> {
        let schema = AssetJsonSchema::create_schema(self.all_assets.all_asset_keys());
        std::fs::write(self.config.get_asset_schema_path(), schema)?;
        Ok(())
    }
}

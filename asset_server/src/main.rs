mod asset;
mod asset_cache;
mod asset_database;
mod asset_loader;
mod asset_sourcer;
mod assets_config;
mod file_change;
mod read_startup;
mod source_files;
use std::{collections::HashMap, fs};

use asset::{Asset, AssetDependency, AssetRef, AssetTypes};
use asset_cache::AssetCompilationFile;
use asset_database::AssetDatabaseMigrated;
use asset_loader::ShaderLoader;
use env_logger::Env;
use source_files::{SourceFileRef, SourceFiles};

use crate::{
    asset_database::AssetDatabase,
    asset_sourcer::{AssetSourcer, CreateAssetInfo, ShaderSourcer},
    assets_config::AssetsConfig,
    source_files::SourceFilesMap,
};

pub enum MyAssetTypes {
    Shader(Asset<ShaderLoader>),
    // Model(Asset<ModelLoader>),
}
impl AssetTypes for MyAssetTypes {
    fn get_key(&self) -> &AssetRef {
        match self {
            MyAssetTypes::Shader(asset) => asset.get_key(),
            // MyAssetTypes::Model(asset) => asset.get_key(),
        }
    }
}

impl MyAssetTypes {
    fn dependencies(&self) -> impl Iterator<Item = &AssetDependency> {
        match self {
            MyAssetTypes::Shader(asset) => asset.dependencies.iter(),
            // MyAssetTypes::Model(asset) => asset.dependencies(),
        }
    }

    fn populate_from_cache_file(&mut self, asset_cache_file: AssetCompilationFile) {
        match self {
            MyAssetTypes::Shader(asset) => asset.populate_from_cache_file(asset_cache_file),
            // MyAssetTypes::Model(asset) => asset.populate_from_cache_file(asset_cache_file),
        }
    }
}

struct Assets {
    assets: HashMap<AssetRef, MyAssetTypes>,
    asset_dependencies_inverse: HashMap<SourceFileRef, Vec<AssetRef>>,
}
impl Assets {
    fn new() -> Self {
        Self {
            assets: HashMap::new(),
            asset_dependencies_inverse: HashMap::new(),
        }
    }

    fn add_asset(&mut self, asset: MyAssetTypes) {
        let key = asset.get_key().clone();
        for dependency in asset.dependencies() {
            self.asset_dependencies_inverse
                .entry(dependency.file.clone())
                .or_default()
                .push(key.clone());
        }
        self.assets.insert(key.clone(), asset);
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();

    let config = AssetsConfig {
        version: 0,
        source: "assets".into(),
        target: "target-assets".into(),
    };

    fs::create_dir_all(&config.target)?;
    let asset_database = load_asset_database(&config)?;

    // TODO: start the file watcher *here*

    // Read the source files and create the assets
    let asset_sourcers: Vec<Box<dyn AssetSourcer<MyAssetTypes>>> = vec![Box::new(ShaderSourcer {})];

    let mut assets = Assets::new();
    let source_files = SourceFilesMap::read_startup(&config, &asset_sourcers);
    for (source_ref, _) in source_files.0.iter() {
        for asset_sourcer in asset_sourcers.iter() {
            if !asset_sourcer.can_potentially_handle(source_ref) {
                continue;
            }
            for mut asset in
                asset_sourcer.create(CreateAssetInfo::from_source_file(source_ref.clone()))
            {
                if let Some(asset_cache_file) = asset_database
                    .get_asset_compilation_file(asset.get_key())
                    .ok()
                    .flatten()
                {
                    asset.populate_from_cache_file(asset_cache_file);
                }
                assets.add_asset(asset);
            }
        }
    }

    // TODO: Start working with the file watcher channel
    let _source_files = SourceFiles::new(source_files);

    println!("Hello, world!");

    Ok(())
}

fn load_asset_database(
    config: &AssetsConfig,
) -> anyhow::Result<AssetDatabase<AssetDatabaseMigrated>> {
    let database_config = redb::Builder::new();

    let mut asset_database =
        AssetDatabase::new(database_config.create(config.get_asset_cache_db_path())?);
    if asset_database.needs_migration(config.version) {
        std::mem::drop(asset_database);
        fs::remove_dir_all(&config.target)?;
        fs::create_dir_all(&config.target)?;
        asset_database =
            AssetDatabase::new(database_config.create(config.get_asset_cache_db_path())?);
    }
    Ok(asset_database.finished_migration())
}

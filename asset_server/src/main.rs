mod asset;
mod asset_database;
mod asset_loader;
mod asset_sourcer;
mod assets_config;
mod file_change;
mod read_startup;
mod source_files;
use std::{collections::HashMap, fs};

use asset::{Asset, AssetRef};
use asset_database::AssetDatabaseMigrated;
use env_logger::Env;
use source_files::{SourceFileRef, SourceFiles};

use crate::{
    asset::AssetCache,
    asset_database::AssetDatabase,
    asset_sourcer::{AssetSourcer, CreateAssetInfo, ShaderSourcer},
    assets_config::AssetsConfig,
    source_files::SourceFilesMap,
};

struct Assets {
    assets: HashMap<AssetRef, Asset>,
    asset_dependencies_inverse: HashMap<SourceFileRef, Vec<AssetRef>>,
}
impl Assets {
    fn new() -> Self {
        Self {
            assets: HashMap::new(),
            asset_dependencies_inverse: HashMap::new(),
        }
    }

    fn add_asset(&mut self, asset: Asset) {
        for dependency in asset.dependencies.iter() {
            self.asset_dependencies_inverse
                .entry(dependency.file.clone())
                .or_default()
                .push(asset.key.clone());
        }
        self.assets.insert(asset.key.clone(), asset);
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
    let mut asset_database = load_asset_database(&config)?;

    // TODO: start the file watcher *here*

    // Read the source files and create the assets
    let asset_sourcers: Vec<Box<dyn AssetSourcer>> = vec![Box::new(ShaderSourcer {})];

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
                    .get_asset_cache_file(&asset.key)
                    .ok()
                    .flatten()
                {
                    asset.main_file.timestamp = asset_cache_file.main_file.timestamp;
                    assert!(asset.main_file.file == asset_cache_file.main_file.file);
                    asset.dependencies = asset_cache_file.dependencies;
                    asset.cache = AssetCache::File(asset_cache_file.id);
                }
                assets.add_asset(asset);
            }
        }
    }

    // TODO: Start working with the file watcher channel
    let source_files = SourceFiles::new(source_files);

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

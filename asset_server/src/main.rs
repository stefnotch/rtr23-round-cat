mod asset_database;
mod asset_file;
mod asset_sourcer;
mod assets_config;
mod read_startup;
mod source_files;
use std::{collections::HashMap, fs, path::PathBuf, time::SystemTime};

use asset_database::AssetDatabaseMigrated;
use asset_sourcer::{Asset, AssetRef};
use serde::{Deserialize, Serialize};
use source_files::SourceFileRef;
use uuid::Uuid;

use crate::{
    asset_database::AssetDatabase,
    asset_sourcer::{AssetSourcer, CreateAssetInfo, ShaderSourcer},
    assets_config::AssetsConfig,
    read_startup::read_startup,
    source_files::SourceFiles,
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let config = AssetsConfig {
        version: 0,
        source: "assets".into(),
        target: "target-assets".into(),
    };

    let mut asset_database = load_asset_database(&config)?;

    // TODO: start the file watcher *here*

    // Update the source files in the asset caching database
    let asset_sourcers: Vec<Box<dyn AssetSourcer>> = vec![Box::new(ShaderSourcer {})];
    let source_files = read_startup(&config, &asset_sourcers);
    asset_database.set_source_files(source_files)?;

    let mut assets = Assets::new();

    // for (source_ref, _) in source_files.files.iter() {
    // for asset_sourcer in asset_sourcers.iter() {
    // if !asset_sourcer.can_potentially_handle(source_ref) {
    // continue;
    // }
    // for asset in asset_sourcer.create(CreateAssetInfo::from_source_file(source_ref.clone()))
    // {
    // assets.assets.insert(asset.get_key().clone(), asset);
    // }
    // }
    // }

    println!("Hello, world!");

    Ok(())
}

fn load_asset_database(
    config: &AssetsConfig,
) -> anyhow::Result<AssetDatabase<AssetDatabaseMigrated>> {
    fs::create_dir_all(&config.target)?;
    let database_config = sled::Config {
        path: config.get_asset_cache_db_path().into(),
        flush_every_ms: Some(1000),
        ..Default::default()
    };
    let mut asset_database = AssetDatabase::new(sled::Db::open_with_config(&database_config)?);
    if asset_database.needs_migration(config.version) {
        std::mem::drop(asset_database);
        fs::remove_dir_all(&config.target)?;
        fs::create_dir_all(&config.target)?;
        asset_database = AssetDatabase::new(sled::Db::open_with_config(&database_config)?);
    }
    Ok(asset_database.finished_migration())
}

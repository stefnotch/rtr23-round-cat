mod asset_database;
mod asset_file;
mod asset_sourcer;
mod assets_config;
mod read_startup;
mod source_files;
use std::{collections::HashMap, fs, path::PathBuf, time::SystemTime};

use asset_database::AssetDatabaseMigrated;
use asset_sourcer::{Asset, AssetRef};
use env_logger::Env;
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

    fn add_asset(&mut self, asset: Asset) {
        if let Some(asset_file_info) = &asset.cache_file_info {
            for dependency in asset_file_info.dependencies.iter() {
                self.asset_dependencies_inverse
                    .entry(dependency.clone())
                    .or_default()
                    .push(asset.key.clone());
            }
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

    // Update the source files in the asset caching database
    let asset_sourcers: Vec<Box<dyn AssetSourcer>> = vec![Box::new(ShaderSourcer {})];
    let source_files = read_startup(&config, &asset_sourcers);

    let mut assets = Assets::new();

    for (source_ref, _) in source_files.files.iter() {
        for asset_sourcer in asset_sourcers.iter() {
            if !asset_sourcer.can_potentially_handle(source_ref) {
                continue;
            }
            for mut asset in
                asset_sourcer.create(CreateAssetInfo::from_source_file(source_ref.clone()))
            {
                asset.cache_file_info = asset_database
                    .get_asset_file_info(asset.get_key())
                    .ok()
                    .flatten();
                assets.add_asset(asset);
            }
        }
    }

    println!("Hello, world!");

    Ok(())
}

fn load_asset_database(
    config: &AssetsConfig,
) -> anyhow::Result<AssetDatabase<AssetDatabaseMigrated>> {
    let database_config = sled::Config::default()
        .path(config.get_asset_cache_db_path())
        .flush_every_ms(Some(1000));

    let mut asset_database = AssetDatabase::new(database_config.open()?);
    // if asset_database.needs_migration(config.version) {
    //     std::mem::drop(asset_database);
    //     fs::remove_dir_all(&config.target)?;
    //     fs::create_dir_all(&config.target)?;
    //     asset_database = AssetDatabase::new(sled::Db::open_with_config(&database_config)?);
    // }
    Ok(asset_database.finished_migration())
}

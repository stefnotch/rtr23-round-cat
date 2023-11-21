use asset_database::{AssetDatabase, AssetDatabaseMigrated};
use asset_loader::{AssetCollectionLoader, SceneLoader, ShaderLoader};
use asset_server::{load_asset_database, AllAssets, MyAssetServer};
use asset_sourcer::{AssetCollectionSourcer, SceneSourcer, ShaderSourcer};
use assets_config::AssetsConfig;
use source_files::SourceFiles;

pub mod asset;
pub mod asset_compilation;
pub mod asset_database;
pub mod asset_loader;
pub mod asset_server;
pub mod asset_sourcer;
pub mod assets_config;
pub mod file_change;
pub mod json_schema;
pub mod read_startup;
pub mod source_files;

impl MyAssetServer {
    pub fn new(source_files: SourceFiles, db: AssetDatabase<AssetDatabaseMigrated>) -> Self {
        Self {
            source_files,
            asset_sourcers: vec![
                Box::new(ShaderSourcer {}),
                Box::new(AssetCollectionSourcer {}),
                Box::new(SceneSourcer {}),
            ],
            asset_database: db,
            all_assets: AllAssets::new()
                .with_asset_type(ShaderLoader {})
                .with_asset_type(AssetCollectionLoader {})
                .with_asset_type(SceneLoader {}),
        }
    }
}

pub fn create_default_asset_server() -> anyhow::Result<MyAssetServer> {
    let config = AssetsConfig {
        version: 0,
        source: "assets".into(),
        target: "target-assets".into(),
    };

    std::fs::create_dir_all(&config.target)?;

    let asset_database = load_asset_database(&config)?;

    Ok(MyAssetServer::new(
        SourceFiles::new(config.source.clone()),
        asset_database,
    ))
}

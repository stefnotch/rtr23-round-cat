use std::fs;

use asset_common::{
    ipc::{get_ipc_name, ReadWriteLenPrefixed},
    scene::Scene,
    shader::Shader,
    AssetData, AssetRef,
};
use asset_server::{
    asset_database::{AssetDatabase, AssetDatabaseMigrated},
    asset_loader::{SceneLoader, ShaderLoader},
    asset_sourcer::{AssetSourcer, CreateAssetInfo, SceneSourcer, ShaderSourcer},
    assets_config::AssetsConfig,
    source_files::{SourceFiles, SourceFilesMap},
    Assets, MyAssetServer, MyAssetTypes,
};
use env_logger::Env;
use interprocess::local_socket::LocalSocketListener;

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
    let asset_sourcers: Vec<Box<dyn AssetSourcer<MyAssetTypes>>> =
        vec![Box::new(ShaderSourcer {}), Box::new(SceneSourcer {})];

    let mut shader_assets = Assets::new();
    let mut scene_assets = Assets::new();
    let source_files = SourceFilesMap::read_startup(&config, &asset_sourcers);
    for (source_ref, _) in source_files.files.iter() {
        for asset_sourcer in asset_sourcers.iter() {
            if !asset_sourcer.might_read(source_ref) {
                continue;
            }
            for asset in asset_sourcer.create(
                CreateAssetInfo::from_source_file(source_ref.clone()),
                &asset_database,
            ) {
                match asset {
                    MyAssetTypes::Shader(asset) => shader_assets.add_asset(asset),
                    MyAssetTypes::Scene(asset) => scene_assets.add_asset(asset),
                    // MyAssetTypes::Model(asset) => model_assets.add_asset(asset),
                }
            }
        }
    }

    // TODO: Start working with the file watcher channel
    let mut assets_server = MyAssetServer {
        config,
        source_files: SourceFiles::new(source_files),
        asset_database,

        shader_loader: ShaderLoader {},
        shader_assets,

        scene_loader: SceneLoader {},
        scene_assets,
    };

    assets_server.write_schema_file()?;

    let ipc_socket_server = LocalSocketListener::bind(get_ipc_name())?;
    // Only 1 client is supported at a time
    for connection in ipc_socket_server.incoming() {
        let mut connection = connection?;
        loop {
            let buf = connection.read_len_prefixed()?;
            let asset_ref = AssetRef::from_bytes(&buf);
            let buf = connection.read_len_prefixed()?;
            let asset_type_id = std::str::from_utf8(&buf)?;

            match asset_type_id {
                Shader::ID => {
                    let asset_data = assets_server.load_shader_asset(asset_ref)?;
                    let buf = asset_data.to_bytes()?;
                    connection.write_len_prefixed(&buf)?;
                }
                Scene::ID => {
                    let asset_data = assets_server.load_scene_asset(asset_ref)?;
                    let buf = asset_data.to_bytes()?;
                    connection.write_len_prefixed(&buf)?;
                }
                _ => {
                    anyhow::bail!("Unknown asset type id {}", asset_type_id);
                }
            }
        }
    }

    // TODO:
    // - File watcher (+ a changed asset map?)
    // - Error recovery (aka re-request the asset)

    // - When our program starts up, it asks the asset server for the scene.json, and then proceeds to load everything that the scene.json references.
    // In release mode, everything that the scene.json references is pre-compiled and serialised to the disk. And then the released program loads those files from the disk instead of asking the asset server.
    //
    // - Automatically cleaning up the target-assets folder
    // - Gentle shutdown https://rust-cli.github.io/book/in-depth/signals.html

    // TOOD: https://github.com/typst/comemo or https://github.com/Justice4Joffrey/depends-rs or https://github.com/salsa-rs/salsa
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

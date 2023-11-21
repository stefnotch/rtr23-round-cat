use std::fs;

use asset_common::{
    ipc::{get_ipc_name, ReadWriteLenPrefixed},
    scene::Scene,
    shader::Shader,
    AssetData, AssetRef,
};
use asset_server::{
    asset_loader::{SceneLoader, ShaderLoader},
    asset_server::{load_asset_database, MyAssetServer},
    asset_sourcer::{SceneSourcer, ShaderSourcer},
    assets_config::AssetsConfig,
    create_default_asset_server,
    source_files::SourceFiles,
};
use env_logger::Env;
use interprocess::local_socket::LocalSocketListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("warn")).init();

    let mut asset_server = create_default_asset_server()?;

    // TODO: start the file watcher *here*

    // Read the source files and create the assets
    asset_server.load_startup();

    // TODO: Start working with the file watcher channel

    asset_server.write_schema_file()?;

    let ipc_socket_server = LocalSocketListener::bind(get_ipc_name())?;
    // Only 1 client is supported at a time
    for connection in ipc_socket_server.incoming() {
        let mut connection = connection?;
        loop {
            let buf = connection.read_len_prefixed()?;
            let asset_ref = AssetRef::from_bytes(&buf);
            let buf = connection.read_len_prefixed()?;
            let asset_type_id = std::str::from_utf8(&buf)?;

            if asset_type_id == Shader::id() {
                let asset_data = asset_server.load_asset::<Shader>(asset_ref)?;
                let buf = asset_data.to_bytes()?;
                connection.write_len_prefixed(&buf)?;
            } else if asset_type_id == Scene::id() {
                let asset_data = asset_server.load_asset::<Scene>(asset_ref)?;
                let buf = asset_data.to_bytes()?;
                connection.write_len_prefixed(&buf)?;
            } else {
                anyhow::bail!("Unknown asset type id {}", asset_type_id);
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

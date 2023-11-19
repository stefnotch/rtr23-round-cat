mod asset;
mod asset_cache;
mod asset_database;
mod asset_loader;
mod asset_sourcer;
mod assets_config;
mod file_change;
mod json_schema;
mod read_startup;
mod source_files;
use std::{collections::HashMap, fs, sync::Arc};

use asset::{Asset, AssetDependency, AssetRef, AssetType, Shader};
use asset_cache::AssetCompilationFile;
use asset_database::AssetDatabaseMigrated;
use asset_loader::{AssetData, ShaderLoader};
use env_logger::Env;
use json_schema::AssetJsonSchema;
use source_files::{SourceFileRef, SourceFiles};

use crate::{
    asset_database::AssetDatabase,
    asset_sourcer::{AssetSourcer, CreateAssetInfo, ShaderSourcer},
    assets_config::AssetsConfig,
    source_files::SourceFilesMap,
};

pub enum MyAssetTypes {
    Shader(Asset<Shader>),
    // Model(Asset<ModelLoader>),
}
impl MyAssetTypes {
    fn get_key(&self) -> &AssetRef {
        match self {
            MyAssetTypes::Shader(asset) => asset.get_key(),
            // MyAssetTypes::Model(asset) => asset.get_key(),
        }
    }
    fn populate_from_cache_file(&mut self, asset_cache_file: AssetCompilationFile) {
        match self {
            MyAssetTypes::Shader(asset) => asset.populate_from_cache_file(asset_cache_file),
            // MyAssetTypes::Model(asset) => asset.populate_from_cache_file(asset_cache_file),
        }
    }
}

struct Assets<T: AssetData> {
    assets: HashMap<AssetRef, Asset<T>>,
    asset_dependencies_inverse: HashMap<SourceFileRef, Vec<AssetRef>>,
}
impl<T: AssetData> Assets<T> {
    fn new() -> Self {
        Self {
            assets: HashMap::new(),
            asset_dependencies_inverse: HashMap::new(),
        }
    }

    fn add_asset(&mut self, asset: Asset<T>) {
        for dependency in asset.dependencies.iter() {
            self.asset_dependencies_inverse
                .entry(dependency.file.clone())
                .or_default()
                .push(asset.key.clone());
        }
        self.assets.insert(asset.key.clone(), asset);
    }
}

struct AssetsServer {
    config: AssetsConfig,
    source_files: SourceFiles,
    asset_database: AssetDatabase<AssetDatabaseMigrated>,

    shader_loader: ShaderLoader,
    shader_assets: Assets<Shader>,
}

impl AssetsServer {
    fn load_shader_asset(&mut self, request: AssetRef) -> anyhow::Result<Arc<Shader>> {
        let asset = self
            .shader_assets
            .assets
            .get_mut(&request)
            .ok_or_else(|| anyhow::format_err!("Asset not found {:?}", request))?;

        let asset_data = asset.load(
            &self.shader_loader,
            &mut self.asset_database,
            &self.config,
            &self.source_files,
        )?;

        Ok(asset_data)
    }

    fn write_schema_file(&self) -> anyhow::Result<()> {
        let schema = AssetJsonSchema::create_schema(
            self.shader_assets.assets.keys(), // .chain(self.model_assets.assets.keys()
        );
        std::fs::write(self.config.get_asset_schema_path(), schema)?;
        Ok(())
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

    let mut shader_assets = Assets::new();
    let source_files = SourceFilesMap::read_startup(&config, &asset_sourcers);
    for (source_ref, _) in source_files.0.iter() {
        for asset_sourcer in asset_sourcers.iter() {
            if !asset_sourcer.might_read(source_ref) {
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
                match asset {
                    MyAssetTypes::Shader(asset) => shader_assets.add_asset(asset),
                    // MyAssetTypes::Model(asset) => model_assets.add_asset(asset),
                }
            }
        }
    }

    // TODO: Start working with the file watcher channel
    let mut assets_server = AssetsServer {
        config,
        source_files: SourceFiles::new(source_files),
        asset_database,

        shader_loader: ShaderLoader {},
        shader_assets,
    };

    for a in assets_server.shader_assets.assets.iter_mut() {
        println!("{:?}", a.0);
    }

    let test_shader = assets_server.load_shader_asset(AssetRef {
        name: vec!["shaders".into(), "base".into()],
        asset_type: AssetType::Shader,
    });

    assets_server.write_schema_file()?;

    // TODO:
    // - File watcher (+ a changed asset map?)
    // - Error recovery (aka re-request the asset)
    // - Implement an IPC way of requesting assets.

    // - Create a JSON schema file with all virtual asset file names
    // - Add a scene.json file which references everything that we need. When our program starts up, it asks the asset server for the scene.json, and then proceeds to load everything that the scene.json references.
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

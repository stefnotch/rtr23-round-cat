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

use crate::{
    asset::{Asset, AssetRef, Shader},
    asset_database::AssetDatabase,
    asset_database::AssetDatabaseMigrated,
    asset_loader::{AssetData, ShaderLoader},
    asset_sourcer::{AssetSourcer, CreateAssetInfo, ShaderSourcer},
    assets_config::AssetsConfig,
    json_schema::AssetJsonSchema,
    source_files::{SourceFileRef, SourceFiles},
};

pub enum MyAssetTypes {
    Shader(Asset<Shader>),
    // Model(Asset<ModelLoader>),
}

pub struct Assets<T: AssetData> {
    pub assets: HashMap<AssetRef, Asset<T>>,
    pub asset_dependencies_inverse: HashMap<SourceFileRef, Vec<AssetRef>>,
}
impl<T: AssetData> Assets<T> {
    pub fn new() -> Self {
        Self {
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
    pub asset_database: AssetDatabase<AssetDatabaseMigrated>,

    pub shader_loader: ShaderLoader,
    pub shader_assets: Assets<Shader>,
}

impl MyAssetServer {
    pub fn load_shader_asset(&mut self, request: AssetRef) -> anyhow::Result<Arc<Shader>> {
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

    pub fn write_schema_file(&self) -> anyhow::Result<()> {
        let schema = AssetJsonSchema::create_schema(
            self.shader_assets.assets.keys(), // .chain(self.model_assets.assets.keys()
        );
        std::fs::write(self.config.get_asset_schema_path(), schema)?;
        Ok(())
    }
}

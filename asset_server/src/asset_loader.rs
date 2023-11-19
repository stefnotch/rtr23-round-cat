mod shader_loader;

pub use shader_loader::*;

use crate::{
    asset::Asset, asset_cache::AssetCompilationFile, assets_config::AssetsConfig,
    source_files::SourceFiles,
};

pub trait AssetData: Sized {}

pub trait AssetLoader {
    type AssetData: AssetData;

    /// Compiles an asset from source files.
    /// Does not check if compilation is actually necessary.
    fn compile_asset(
        &self,
        asset: &Asset<Self::AssetData>,
        config: &AssetsConfig,
        source_files: &SourceFiles,
    ) -> anyhow::Result<AssetCompileResult<Self::AssetData>>;

    /// Loads an already compiled asset.
    fn load_asset(
        &self,
        compilation_result: &AssetCompilationFile,
        config: &AssetsConfig,
    ) -> anyhow::Result<Self::AssetData>;
}

pub struct AssetCompileResult<Data: AssetData> {
    pub compilation_file: AssetCompilationFile,

    /// A compilation can *optionally* also directly produce the asset data.
    pub data: Option<Data>,
}

/// A temporary file that will be deleted when dropped.
struct TempFile {
    path: Option<std::path::PathBuf>,
}
impl TempFile {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path: Some(path) }
    }
    pub fn keep_file(mut self) -> std::path::PathBuf {
        self.path.take().unwrap()
    }
    pub fn path(&self) -> &std::path::Path {
        self.path.as_ref().unwrap()
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            if let Err(err) = std::fs::remove_file(path) {
                log::warn!("Failed to remove temporary file {:?}: {}", path, err);
            }
        }
    }
}

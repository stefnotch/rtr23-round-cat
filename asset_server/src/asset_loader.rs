mod scene_loader;
mod shader_loader;

use asset_common::AssetData;
pub use scene_loader::*;
pub use shader_loader::*;

use crate::{asset::Asset, asset_compilation::AssetCompilationFile, source_files::SourceFiles};

pub trait AssetLoader {
    type AssetData: AssetData;

    /// Compiles an asset from source files.
    /// Does not check if compilation is actually necessary.
    fn compile_asset(
        &self,
        asset: &Asset<Self::AssetData>,
        source_files: &SourceFiles,
        target_path: &std::path::Path,
    ) -> anyhow::Result<AssetCompileResult<Self::AssetData>>;

    /// Loads an already compiled asset.
    fn load_asset(
        &self,
        compilation_result: &AssetCompilationFile,
        source_files: &SourceFiles,
        target_path: &std::path::Path,
    ) -> anyhow::Result<Self::AssetData>;
}

impl<Loader: AssetLoader + ?Sized> AssetLoader for Box<Loader> {
    type AssetData = Loader::AssetData;

    fn compile_asset(
        &self,
        asset: &Asset<Self::AssetData>,
        source_files: &SourceFiles,
        target_path: &std::path::Path,
    ) -> anyhow::Result<AssetCompileResult<Self::AssetData>> {
        (**self).compile_asset(asset, source_files, target_path)
    }

    fn load_asset(
        &self,
        compilation_result: &AssetCompilationFile,
        source_files: &SourceFiles,
        target_path: &std::path::Path,
    ) -> anyhow::Result<Self::AssetData> {
        (**self).load_asset(compilation_result, source_files, target_path)
    }
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

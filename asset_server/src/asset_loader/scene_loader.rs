use asset_common::scene::Scene;
use uuid::Uuid;

use crate::{asset::Asset, asset_compilation::AssetCompilationFile, source_files::SourceFiles};

use super::{AssetCompileResult, AssetLoader};

pub struct SceneLoader {}

impl AssetLoader for SceneLoader {
    type AssetData = Scene;

    fn compile_asset(
        &self,
        asset: &Asset<Self::AssetData>,
        source_files: &SourceFiles,
        target_path: &std::path::Path,
    ) -> anyhow::Result<AssetCompileResult<Self::AssetData>> {
        let files_snapshot = source_files.take_snapshot();
        let data = files_snapshot.read(&asset.main_file.file)?;

        Ok(AssetCompileResult {
            compilation_file: AssetCompilationFile {
                main_file: asset.main_file.clone(),
                dependencies: Default::default(),
                id: Uuid::new_v4(),
            },
            data: Some(Scene { data }),
        })
    }

    fn load_asset(
        &self,
        compilation_result: &AssetCompilationFile,
        source_files: &SourceFiles,
        target_path: &std::path::Path,
    ) -> anyhow::Result<Self::AssetData> {
        let files_snapshot = source_files.take_snapshot();
        let file = &compilation_result.main_file.file;
        let data = files_snapshot.read(file)?;
        Ok(Scene { data })
    }
}

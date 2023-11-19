use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::bail;
use uuid::Uuid;

use crate::{
    asset::{Asset, AssetDependency, Shader},
    asset_cache::AssetCompilationFile,
    asset_loader::TempFile,
    assets_config::AssetsConfig,
    source_files::SourceFiles,
};

use super::{AssetCompileResult, AssetLoader};

pub struct ShaderLoader {}

impl ShaderLoader {
    fn get_output_path(id: &Uuid, config: &AssetsConfig) -> PathBuf {
        config.target.join(id.to_string()).with_extension("spv")
    }
}

impl AssetLoader for ShaderLoader {
    type AssetData = Shader;

    fn compile_asset(
        &self,
        asset: &Asset<Self::AssetData>,
        config: &AssetsConfig,
        source_files: &SourceFiles,
    ) -> anyhow::Result<AssetCompileResult<Self::AssetData>> {
        let snapshot_lock = source_files.take_snapshot();
        log::info!("Loading asset {:?}", asset.key);

        let id = Uuid::new_v4();
        let input_path = asset.main_file_path(config);
        let output_path = TempFile::new(ShaderLoader::get_output_path(&id, config));
        let output_d_path = TempFile::new(output_path.path().with_extension("spv.d"));

        let shader_compile_result = Command::new("glslc")
            .arg("-c") // Compile the shader
            .arg("-MD") // And also generate makefile dependencies
            .arg(&input_path)
            .arg("-o")
            .arg(output_path.path())
            .arg("-MT") // And simplify the makefile dependency file
            .arg("shader")
            .status()?;

        if !shader_compile_result.success() {
            bail!(
                "Shader compilation for {} failed: {}",
                asset.main_file.file.get_path(),
                shader_compile_result
            );
        }

        // It also generates a .d file, which we need to read to get the dependencies
        let output_d = std::fs::read_to_string(output_d_path.path())?;
        let dependency_paths = output_d
            .strip_prefix("shader:")
            .ok_or_else(|| anyhow::format_err!("Invalid dependency file for {:?}", asset.key))?
            .trim()
            .split(' ')
            .map(|path| config.get_source_file_ref(Path::new(path)));

        let mut asset_dependencies = HashSet::new();
        for dependency in dependency_paths {
            let timestamp = source_files.get(&snapshot_lock, &dependency)?;
            asset_dependencies.insert(AssetDependency {
                file: dependency,
                timestamp,
            });
        }

        // We need this part of the compilation results, so we keep it around.
        output_path.keep_file();

        Ok(AssetCompileResult {
            compilation_file: AssetCompilationFile {
                main_file: AssetDependency {
                    file: asset.main_file.file.clone(),
                    timestamp: source_files.get(&snapshot_lock, &asset.main_file.file)?,
                },
                dependencies: asset_dependencies,
                id,
            },
            data: None,
        })
    }

    fn load_asset(
        &self,
        compilation_result: &AssetCompilationFile,
        config: &AssetsConfig,
    ) -> anyhow::Result<Self::AssetData> {
        let output_path = ShaderLoader::get_output_path(&compilation_result.id, config);
        let data = std::fs::read(output_path)?;
        Ok(Shader { data })
    }
}

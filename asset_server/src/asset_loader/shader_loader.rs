use std::{collections::HashSet, path::Path, process::Command};

use anyhow::bail;
use uuid::Uuid;

use crate::{
    asset::{Asset, AssetDependency},
    asset_cache::AssetCompilationFile,
    asset_database::{AssetDatabase, AssetDatabaseMigrated},
    asset_loader::FileDropper,
    assets_config::AssetsConfig,
    source_files::SourceFiles,
};

use super::{AssetCompileResult, AssetLoader};

pub struct ShaderLoader {}

impl AssetLoader for ShaderLoader {
    type AssetData = Vec<u8>;

    fn compile_asset(
        &self,
        asset: &Asset<Self>,
        config: &AssetsConfig,
        source_files: &SourceFiles,
    ) -> anyhow::Result<AssetCompileResult<Self>> {
        let snapshot_lock = source_files.take_snapshot();
        log::info!("Loading asset {:?}", asset.key);

        let id = Uuid::new_v4();
        let input_path = asset.main_file_path(config);
        let output_name = id.to_string();
        let output_path = FileDropper::new(config.target.join(&output_name).with_extension("spv"));
        let output_d_path = FileDropper::new(config.target.join(&output_name).with_extension("d"));

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
            .split(" ")
            .map(|path| config.get_source_file_ref(&Path::new(path)));

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
}

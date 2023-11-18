use std::{path::Path, process::Command, sync::Arc};

use anyhow::bail;
use uuid::Uuid;

use crate::{
    asset::{Asset, AssetCache, AssetData, AssetDependency},
    asset_database::{AssetDatabase, AssetDatabaseMigrated},
    assets_config::AssetsConfig,
    file_change::FileTimestamp,
};

pub fn load_asset(
    asset: &Asset,
    asset_database: &mut AssetDatabase<AssetDatabaseMigrated>,
    config: &AssetsConfig,
    source_files: &mut crate::source_files::SourceFiles,
) -> anyhow::Result<AssetLoadResult> {
    let snapshot_lock = source_files.take_snapshot();
    log::info!("Loading asset {:?}", asset.key);

    let mut asset_cache = asset.cache.clone();
    let mut asset_dependencies;
    let mut asset_data;

    match asset.key.asset_type {
        crate::asset::AssetType::Shader => {
            let input_path = asset
                .main_file
                .file
                .get_path()
                .to_path(config.source.clone());
            let output_name = asset_cache
                .get_file_id()
                .unwrap_or_else(|| {
                    let id = Uuid::new_v4();
                    asset_cache = crate::asset::AssetCache::File(id);
                    id
                })
                .to_string();
            let output_path = config.target.join(&output_name).with_extension("spv");
            let output_d_path = config.target.join(&output_name).with_extension("d");

            let shader_compile_result = Command::new("glslc")
                .arg("-c") // Compile the shader
                .arg("-MD") // And also generate makefile dependencies
                .arg(&input_path)
                .arg("-o")
                .arg(&output_path)
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
            let output_d = std::fs::read_to_string(&output_d_path)?;
            std::fs::remove_file(&output_d_path)?;
            let dependency_paths = output_d
                .strip_prefix("shader:")
                .ok_or_else(|| anyhow::format_err!("Invalid dependency file for {:?}", asset.key))?
                .split(" ")
                .map(|path| config.get_source_file_ref(&Path::new(path)));

            asset_dependencies = Vec::new();
            for dependency in dependency_paths {
                let timestamp = source_files.get(&snapshot_lock, &dependency)?;
                asset_dependencies.push(AssetDependency {
                    file: dependency,
                    timestamp,
                });
            }

            asset_data = Some(Arc::new(AssetData::Shader(std::fs::read(output_path)?)));
        }
        crate::asset::AssetType::Model => todo!(),
    };

    Ok(AssetLoadResult {
        main_file_timestamp: source_files.get(&snapshot_lock, &asset.main_file.file)?,
        dependencies: asset_dependencies,
        cache: asset_cache,
        data: asset_data,
    })
}

pub struct AssetLoadResult {
    pub main_file_timestamp: FileTimestamp,
    pub dependencies: Vec<AssetDependency>,
    pub cache: AssetCache,
    pub data: Option<Arc<AssetData>>,
}

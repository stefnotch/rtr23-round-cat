use std::{path::Path, process::Command};

use uuid::Uuid;

use crate::{
    asset::Asset,
    asset_database::{AssetDatabase, AssetDatabaseMigrated},
    assets_config::AssetsConfig,
};

pub fn load_asset(
    asset: &mut Asset,
    asset_database: &mut AssetDatabase<AssetDatabaseMigrated>,
    config: &AssetsConfig,
) {
    match asset.key.asset_type {
        crate::asset::AssetType::Shader => {
            // TODO: Snapshot the source files (their timestamps) here

            let input_path = asset
                .main_file
                .file
                .get_path()
                .to_path(config.source.clone());
            let output_name = asset
                .cache
                .get_file_id()
                .unwrap_or_else(|| {
                    let id = Uuid::new_v4();
                    asset.cache = crate::asset::AssetCache::File(id);
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
                .status()
                .unwrap();

            if !shader_compile_result.success() {
                log::error!(
                    "Shader compilation for {} failed: {}",
                    asset.main_file.file.get_path(),
                    shader_compile_result
                );
                todo!(); // TODO: Handle this error
            }

            // It also generates a .d file, which we need to read to get the dependencies
            let output_d = std::fs::read_to_string(&output_d_path).unwrap();
            let dependency_paths = output_d
                .strip_prefix("shader:")
                .unwrap()
                .split(" ")
                .map(|path| config.get_source_file_ref(&Path::new(path)));

            // TODO: And use the snapshotted source files here
            asset.dependencies
        }
        crate::asset::AssetType::Model => todo!(),
    }
}

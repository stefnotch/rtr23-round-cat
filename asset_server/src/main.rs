mod asset_sourcer;
mod assets_config;
mod read_startup;
mod source_files;
use std::{collections::HashMap, fs, path::PathBuf, time::SystemTime};

use serde::{Deserialize, Serialize};

use crate::{
    asset_sourcer::{AssetSourcer, CreateAssetInfo, ShaderSourcer},
    assets_config::AssetsConfig,
    read_startup::read_startup,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AssetsConfig {
        source: "assets".into(),
        target: "target-assets".into(),
    };

    let asset_sourcers: Vec<Box<dyn AssetSourcer>> = vec![Box::new(ShaderSourcer {})];

    let old_source_files = fs::read(config.get_source_file_path())
        .ok()
        .and_then(|v| bincode::deserialize(&v).ok())
        .unwrap_or_default();
    let source_files = read_startup(&config, &old_source_files, &asset_sourcers);
    fs::create_dir_all(&config.target)?;
    fs::write(
        config.get_source_file_path(),
        bincode::serialize(&source_files)?,
    )?;

    for (source_ref, source_data) in source_files.files.iter() {
        for asset_sourcer in asset_sourcers.iter() {
            if asset_sourcer.can_potentially_handle(source_ref) {
                let assets =
                    asset_sourcer.create(CreateAssetInfo::from_source_file(source_ref.clone()));
            }
        }
    }

    println!("Hello, world!");

    Ok(())
}

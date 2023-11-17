mod shader_sourcer;

use serde::{Deserialize, Serialize};
pub use shader_sourcer::*;
use std::{path::Path, sync::Arc};

use uuid::Uuid;

use crate::{asset_file::AssetFileInfo, source_files::SourceFileRef};

pub trait AssetSourcer {
    /// Rough filtering for files.
    /// Concrete checks are done later.
    fn can_potentially_handle(&self, path: &SourceFileRef) -> bool;

    fn create(&self, create_info: CreateAssetInfo) -> Vec<Asset>;
}

pub struct CreateAssetInfo {
    pub file_ref: SourceFileRef,
    pub asset_name_base: Vec<String>,
}
impl CreateAssetInfo {
    pub fn from_source_file(file_ref: SourceFileRef) -> Self {
        let asset_name_base = file_ref
            .get_path()
            .with_extension("")
            .components()
            .map(|v| v.as_str().into())
            .collect();
        Self {
            file_ref,
            asset_name_base,
        }
    }
}

/// A reference to an asset.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct AssetRef {
    pub name: Vec<String>,
    pub asset_type: AssetType,
}
/// A lazily loaded asset.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub key: AssetRef,
    main_file: SourceFileRef,

    cache_file_info: Option<AssetFileInfo>,
    data: Option<Arc<AssetData>>,
}

impl Asset {
    pub fn new(key: AssetRef, main_file: SourceFileRef) -> Self {
        Self {
            key,
            main_file,
            cache_file_info: None,
            data: None,
        }
    }

    pub fn get_key(&self) -> &AssetRef {
        &self.key
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub enum AssetType {
    Shader,
    Model,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AssetData {
    Shader(),
    Model(),
}

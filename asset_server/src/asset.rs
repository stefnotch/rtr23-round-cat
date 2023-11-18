use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{file_change::FileTimestamp, source_files::SourceFileRef};

/// A reference to an asset.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct AssetRef {
    pub name: Vec<String>,
    pub asset_type: AssetType,
}
/// A lazily loaded asset.
#[derive(Clone, Debug)]
pub struct Asset {
    pub key: AssetRef,
    pub main_file: AssetDependency,
    /// Can also reference currently nonexistent files.
    /// Main file is implicitly included.
    pub dependencies: Vec<AssetDependency>,

    pub cache: AssetCache,
    pub data: Option<Arc<AssetData>>,
}

impl Asset {
    pub fn new(key: AssetRef, main_file: AssetDependency) -> Self {
        Self {
            key,
            main_file,
            dependencies: vec![],
            cache: AssetCache::InMemory,
            data: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub enum AssetType {
    Shader,
    Model,
}

/// Loaded asset data.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AssetData {
    Shader(Vec<u8>),
    Model(),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetDependency {
    pub file: SourceFileRef,
    pub timestamp: FileTimestamp,
}

#[derive(Clone, Debug)]
pub enum AssetCache {
    File(Uuid),
    InMemory,
}

impl AssetCache {
    pub fn get_file_id(&self) -> Option<Uuid> {
        match self {
            AssetCache::File(id) => Some(*id),
            AssetCache::InMemory => None,
        }
    }
}

impl Default for AssetCache {
    fn default() -> Self {
        Self::InMemory
    }
}

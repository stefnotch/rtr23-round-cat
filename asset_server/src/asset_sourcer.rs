mod shader_sourcer;

use serde::{Deserialize, Serialize};
pub use shader_sourcer::*;
use std::{path::Path, sync::Arc};

use uuid::Uuid;

use crate::{asset::Asset, source_files::SourceFileRef};

pub trait AssetSourcer<AssetTypes> {
    /// Rough filtering for files.
    /// Concrete checks are done later.
    fn can_potentially_handle(&self, path: &SourceFileRef) -> bool;

    fn create(&self, create_info: CreateAssetInfo) -> Vec<AssetTypes>;
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

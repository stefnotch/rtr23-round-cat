mod shader_sourcer;

pub use shader_sourcer::*;
use std::{path::Path, sync::Arc};

use uuid::Uuid;

use crate::source_files::SourceFileRef;

pub trait AssetSourcer {
    /// Rough filtering for files.
    /// Concrete checks are done later.
    fn can_potentially_handle(&self, path: &SourceFileRef) -> bool;

    fn create(&self, create_info: CreateAssetInfo) -> Vec<Asset>;
}

pub struct CreateAssetInfo {
    pub file_ref: SourceFileRef,
    pub asset_ref_base: AssetRef,
}
impl CreateAssetInfo {
    pub fn from_source_file(file_ref: SourceFileRef) -> Self {
        let asset_ref_base = AssetRef(
            file_ref
                .get_path()
                .components()
                .map(|v| v.as_str().into())
                .collect(),
        );
        Self {
            file_ref,
            asset_ref_base,
        }
    }
}

pub struct AssetRef(Vec<String>);

/// A lazily loaded asset.
pub struct Asset {
    pub name: AssetRef,
    pub asset_type: AssetType,
    main_file: SourceFileRef,

    // could also be a generational index?
    // or a hash of the file?
    // or we could store this in a meta file next to the asset?
    // well, I have no special requirements, so this is good
    output_file_id: Uuid,
    /// Can also reference currently nonexistent files.
    extra_files: Vec<SourceFileRef>,
    data: Option<Arc<AssetData>>,
}

impl Asset {
    pub fn new(name: AssetRef, asset_type: AssetType, main_file: SourceFileRef) -> Self {
        Self {
            name,
            asset_type,
            main_file,
            output_file_id: Uuid::new_v4(),
            extra_files: vec![],
            data: None,
        }
    }
}

pub enum AssetType {
    Shader,
    Model,
}

pub enum AssetData {
    Shader(),
    Model(),
}

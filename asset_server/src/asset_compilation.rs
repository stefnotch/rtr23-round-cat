use std::collections::HashSet;

use asset_common::AssetData;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::asset::{Asset, AssetDependency};

/// References a generated asset file
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetCompilationFile {
    pub main_file: AssetDependency,

    /// Can also reference currently nonexistent files.
    pub dependencies: HashSet<AssetDependency>,

    // could also be a generational index?
    // or a hash of the file?
    // or we could store this in a meta file next to the asset?
    // well, I have no special requirements, so this is good
    pub id: Uuid,
}

impl AssetCompilationFile {
    pub fn is_outdated<T: AssetData>(&self, asset: &Asset<T>) -> bool {
        self.main_file.timestamp != asset.main_file.timestamp
            || self.dependencies != asset.dependencies
    }
}

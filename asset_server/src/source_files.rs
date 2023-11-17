use std::{collections::HashMap, time::SystemTime};

use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};

/// Relative to the asset folder root.
#[derive(Serialize, Deserialize, Eq, Hash, PartialEq, Clone)]
pub struct SourceFileRef(RelativePathBuf);
impl SourceFileRef {
    pub fn new(path: RelativePathBuf) -> Self {
        Self(path)
    }

    pub fn get_path(&self) -> &RelativePathBuf {
        &self.0
    }
}

#[derive(Serialize, Deserialize)]
pub struct SourceFiles {
    pub version: u64,
    pub files: HashMap<SourceFileRef, SourceFileData>,
}
impl SourceFiles {
    pub fn new() -> Self {
        Self {
            version: 0,
            files: Default::default(),
        }
    }
}
impl Default for SourceFiles {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize)]
pub struct SourceFileData {
    pub last_changed: Option<SystemTime>,
    pub is_dirty: bool,
}

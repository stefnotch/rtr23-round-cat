use std::{collections::HashMap, time::SystemTime};

use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};

/// Relative to the asset folder root.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
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
    pub files: HashMap<SourceFileRef, SourceFileData>,
}
impl SourceFiles {
    pub fn new() -> Self {
        Self {
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
}

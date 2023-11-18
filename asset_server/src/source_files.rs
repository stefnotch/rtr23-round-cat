use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};

use crate::file_change::FileTimestamp;

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

pub struct SourceFiles {
    pub files: Arc<Mutex<im::HashMap<SourceFileRef, SourceFileData>>>,
    // TODO: Changed file channel https://docs.rs/crossbeam/0.8.2/crossbeam/channel/index.html
}
impl SourceFiles {
    pub fn new(files: HashMap<SourceFileRef, SourceFileData>) -> Self {
        Self {
            files: Arc::new(Mutex::new(im::HashMap::from(files))),
        }
    }
}
#[derive(Clone, Debug)]
pub struct SourceFileData {
    pub timestamp: FileTimestamp,
}

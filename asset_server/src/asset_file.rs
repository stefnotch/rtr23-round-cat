use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::source_files::SourceFileRef;

/// A generated asset file
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetFileInfo {
    /// Can also reference currently nonexistent files.
    pub dependencies: Vec<SourceFileRef>,
    pub timestamp: SystemTime,

    // could also be a generational index?
    // or a hash of the file?
    // or we could store this in a meta file next to the asset?
    // well, I have no special requirements, so this is good
    pub id: Uuid,
}

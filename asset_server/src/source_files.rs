use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{atomic::AtomicU64, Arc, Mutex},
};

use relative_path::{PathExt, RelativePathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::file_change::FileTimestamp;

/// Relative to the asset folder root.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, Hash, PartialEq)]
pub struct SourceFileRef(RelativePathBuf);
impl SourceFileRef {
    pub fn new<P: AsRef<Path>>(path: impl Into<PathBuf>, source_path: P) -> Self {
        let path = path.into();
        Self(
            path.relative_to(source_path.as_ref())
                .unwrap_or_else(|error| {
                    panic!(
                        "Failed to get relative path for {:?} with base {:?}, because of {:?}",
                        path,
                        source_path.as_ref(),
                        error
                    )
                }),
        )
    }

    pub fn get_path(&self) -> &RelativePathBuf {
        &self.0
    }
}

#[derive(Clone, Debug)]
pub struct SourceFiles {
    inner: Arc<SourceFilesInner>,
}
impl SourceFiles {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            inner: Arc::new(SourceFilesInner {
                base_path,
                snapshot_version: AtomicU64::new(0),
                files: Mutex::new(HashMap::new()),
                changed_files: Mutex::new(HashSet::new()),
            }),
        }
    }

    pub fn take_snapshot(&self) -> FilesSnapshot {
        FilesSnapshot {
            version: self
                .inner
                .snapshot_version
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            source_files: self.inner.clone(),
        }
    }

    pub fn update(&self, update: SourceFileUpdate) {
        let file = update.get_file().clone();
        let mut files = self.inner.files.lock().unwrap();
        match update {
            SourceFileUpdate::Insert(file, timestamp) => {
                files.insert(
                    file,
                    SourceFileData {
                        timestamp,
                        snapshot_version: self
                            .inner
                            .snapshot_version
                            .load(std::sync::atomic::Ordering::SeqCst),
                    },
                );
            }
            SourceFileUpdate::Remove(file) => {
                files.remove(&file);
            }
        };
        self.inner.changed_files.lock().unwrap().insert(file);
    }

    pub fn try_take_changed(&self) -> Option<SourceFileRef> {
        let mut changed_files = self.inner.changed_files.lock().unwrap();
        let file = changed_files.iter().next().cloned()?;
        changed_files.remove(&file);
        Some(file)
    }
}

#[derive(Clone, Debug)]
pub enum SourceFileUpdate {
    Insert(SourceFileRef, FileTimestamp),
    Remove(SourceFileRef),
}

impl SourceFileUpdate {
    pub fn get_file(&self) -> &SourceFileRef {
        match self {
            SourceFileUpdate::Insert(file, _) => file,
            SourceFileUpdate::Remove(file) => file,
        }
    }
}

pub struct FilesSnapshot {
    version: u64,
    source_files: Arc<SourceFilesInner>,
}

impl FilesSnapshot {
    pub fn base_path(&self) -> &Path {
        &self.source_files.base_path
    }

    pub fn get(&self, file: &SourceFileRef) -> Result<FileTimestamp, SnapshotReadError> {
        let files = self.source_files.files.lock().unwrap();
        let file = files.get(file).ok_or(SnapshotReadError::NotFound)?;
        if file.snapshot_version <= self.version {
            Ok(file.timestamp)
        } else {
            Err(SnapshotReadError::VersionChanged)
        }
    }

    pub fn read(&self, file: &SourceFileRef) -> Result<Vec<u8>, SnapshotReadError> {
        let data = std::fs::read(file.get_path().to_path(self.base_path()))?;
        // TODO: Technically, this isn't race condition free
        // The fs watcher could still be reporting the old timestamp, despite the file having changed
        let _ = self.get(file)?; // check version after read
        Ok(data)
    }
}

#[derive(Error, Debug)]
pub enum SnapshotReadError {
    #[error("The file was changed while reading.")]
    VersionChanged,
    #[error("The file was not found.")]
    NotFound,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug)]
struct SourceFilesInner {
    base_path: PathBuf,
    /// Every time we want to read multiple, consistent values from the DB, we increment the snapshot_version.
    /// (Similar idea as optimistic locking.)
    snapshot_version: AtomicU64,
    files: Mutex<HashMap<SourceFileRef, SourceFileData>>,
    changed_files: Mutex<HashSet<SourceFileRef>>,
}

#[derive(Clone, Debug)]
struct SourceFileData {
    timestamp: FileTimestamp,
    snapshot_version: u64,
}

use std::{
    collections::HashMap,
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

pub struct SourceFilesMap {
    pub base_path: PathBuf,
    pub files: HashMap<SourceFileRef, FileTimestamp>,
}
impl SourceFilesMap {
    pub fn new(base_path: PathBuf, files: HashMap<SourceFileRef, FileTimestamp>) -> Self {
        Self { base_path, files }
    }
}

#[derive(Clone, Debug)]
pub struct SourceFiles {
    inner: Arc<SourceFilesInner>,
    // TODO: Changed file channel https://docs.rs/crossbeam/0.8.2/crossbeam/channel/index.html
}
impl SourceFiles {
    pub fn new(files_map: SourceFilesMap) -> Self {
        let snapshot_version = 0;
        let files = files_map
            .files
            .into_iter()
            .map(|(file_ref, timestamp)| {
                (
                    file_ref,
                    SourceFileData {
                        timestamp,
                        snapshot_version,
                    },
                )
            })
            .collect();
        Self {
            inner: Arc::new(SourceFilesInner {
                base_path: files_map.base_path,
                snapshot_version: AtomicU64::new(snapshot_version),
                files: Mutex::new(files),
            }),
        }
    }

    pub fn take_snapshot(&self) -> SnapshotLock {
        SnapshotLock(
            self.inner
                .snapshot_version
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        )
    }

    pub fn base_path(&self) -> &Path {
        &self.inner.base_path
    }

    pub fn get(
        &self,
        lock: &SnapshotLock,
        file: &SourceFileRef,
    ) -> Result<FileTimestamp, SnapshotReadError> {
        let files = self.inner.files.lock().unwrap();
        let file = files.get(file).ok_or(SnapshotReadError::NotFound)?;
        if file.snapshot_version <= lock.0 {
            Ok(file.timestamp)
        } else {
            Err(SnapshotReadError::VersionChanged)
        }
    }
}

pub struct SnapshotLock(u64);

#[derive(Error, Debug)]
pub enum SnapshotReadError {
    #[error("The file was changed while reading.")]
    VersionChanged,
    #[error("The file was not found.")]
    NotFound,
}

#[derive(Debug)]
struct SourceFilesInner {
    base_path: PathBuf,
    /// Every time we want to read multiple, consistent values from the DB, we increment the snapshot_version.
    /// (Similar idea as optimistic locking.)
    snapshot_version: AtomicU64,
    files: Mutex<HashMap<SourceFileRef, SourceFileData>>,
}

#[derive(Clone, Debug)]
struct SourceFileData {
    timestamp: FileTimestamp,
    snapshot_version: u64,
}

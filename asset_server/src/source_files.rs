use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc, LockResult, Mutex, MutexGuard},
};

use relative_path::RelativePathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

pub struct SourceFilesMap(pub HashMap<SourceFileRef, FileTimestamp>);
impl SourceFilesMap {
    pub fn new(files: HashMap<SourceFileRef, FileTimestamp>) -> Self {
        Self(files)
    }
}

#[derive(Clone, Debug)]
pub struct SourceFiles {
    inner: Arc<SourceFilesInner>,
    // TODO: Changed file channel https://docs.rs/crossbeam/0.8.2/crossbeam/channel/index.html
}
impl SourceFiles {
    pub fn new(files: SourceFilesMap) -> Self {
        let snapshot_version = 0;
        let files = files
            .0
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

    pub fn get(
        &self,
        lock: &SnapshotLock,
        file: &SourceFileRef,
    ) -> Result<FileTimestamp, SnapshotReadError> {
        let files = self.inner.files.lock().unwrap();
        let file = files.get(file).ok_or_else(|| SnapshotReadError::NotFound)?;
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

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

// The default Hash implementation still works, since we still uphold [the important property](https://doc.rust-lang.org/std/hash/trait.Hash.html#hash-and-eq).
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Hash)]
pub enum FileTimestamp {
    /// Remember that filesystem timestamps are not reliable.
    /// For example, if you copy a file, the timestamp will be the same.
    /// So it's possible for a user to copy an old file around, and then
    /// the asset server is going to see a timestamp that's clearly in the past.
    ///
    /// So we shouldn't ever check for an ordering, instead we check for equality!
    Timestamp(SystemTime),

    /// Always un-equal to any other timestamp.
    Unknown,
}

impl FileTimestamp {
    pub fn new(timestamp: SystemTime) -> Self {
        Self::Timestamp(timestamp)
    }

    pub fn unknown() -> Self {
        Self::Unknown
    }
}

impl PartialEq for FileTimestamp {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Timestamp(l0), Self::Timestamp(r0)) => l0 == r0,
            (Self::Unknown, _) => false,
            (_, Self::Unknown) => false,
        }
    }
}

impl Eq for FileTimestamp {}

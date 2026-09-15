use std::time::{SystemTime, UNIX_EPOCH};

use russh_sftp::client::fs::Metadata as RemoteMetadata;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FileSignature {
    pub(super) is_directory: bool,
    pub(super) size: u64,
    pub(super) modified_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SyncDecision {
    Changed,
    Unchanged,
}

impl FileSignature {
    pub(super) fn from_local(metadata: &std::fs::Metadata) -> Self {
        Self {
            is_directory: metadata.is_dir(),
            size: metadata.len(),
            modified_at: metadata.modified().ok().and_then(system_time_seconds),
        }
    }

    pub(super) fn from_remote(metadata: &RemoteMetadata) -> Self {
        Self {
            is_directory: metadata.is_dir(),
            size: metadata.len(),
            modified_at: metadata.mtime.map(u64::from),
        }
    }

    pub(super) fn compare(&self, other: &Self) -> SyncDecision {
        if self.is_directory != other.is_directory {
            return SyncDecision::Changed;
        }
        if self.is_directory {
            return SyncDecision::Unchanged;
        }
        if self.size == other.size
            && self.modified_at.is_some()
            && self.modified_at == other.modified_at
        {
            SyncDecision::Unchanged
        } else {
            SyncDecision::Changed
        }
    }
}

pub(super) fn system_time_seconds(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

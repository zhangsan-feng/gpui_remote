use std::{path::PathBuf, time::SystemTime};

#[derive(Clone, Debug)]
pub(crate) struct LocalEntry {
    pub(crate) name: String,
    pub(crate) path: PathBuf,
    pub(crate) is_directory: bool,
    pub(crate) size: u64,
    pub(crate) modified_at: Option<SystemTime>,
}

#[derive(Clone, Debug)]
pub(crate) struct SftpEntry {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) is_directory: bool,
    pub(crate) size: u64,
    pub(crate) modified_at: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FileSignature {
    pub(crate) is_directory: bool,
    pub(crate) size: u64,
    pub(crate) modified_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyncDecision {
    Changed,
    Unchanged,
}

impl FileSignature {
    pub(crate) fn new(is_directory: bool, size: u64, modified_at: Option<u64>) -> Self {
        Self {
            is_directory,
            size,
            modified_at,
        }
    }

    pub(crate) fn compare(&self, other: &Self) -> SyncDecision {
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

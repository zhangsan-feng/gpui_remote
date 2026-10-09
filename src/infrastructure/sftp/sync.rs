use std::time::UNIX_EPOCH;

use russh_sftp::client::fs::Metadata as RemoteMetadata;

pub(super) use crate::domain::sftp::{FileSignature, SyncDecision};

pub(super) fn from_remote(metadata: &RemoteMetadata) -> FileSignature {
    FileSignature::new(
        metadata.is_dir(),
        metadata.len(),
        metadata.mtime.map(u64::from),
    )
}

pub(super) fn from_local(metadata: &std::fs::Metadata) -> FileSignature {
    FileSignature::new(
        metadata.is_dir(),
        metadata.len(),
        metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs()),
    )
}

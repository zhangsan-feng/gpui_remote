use std::time::UNIX_EPOCH;

use crate::domain::terminal::TerminalStatus;

use super::super::state::{
    LocalSnapshot, SftpDirectorySummary, SftpEntrySummary, SftpSnapshot, SftpTransferInfo,
    SftpWorkspaceData, SftpWorkspaceSnapshot, TransferRecord, TransferRequest,
};

pub(super) fn to_workspace_snapshot(state: &SftpWorkspaceData) -> SftpWorkspaceSnapshot {
    SftpWorkspaceSnapshot {
        remote: map_remote_directory(&state.remote),
        local: map_local_directory(&state.local),
        transfers: state.transfers.iter().map(map_transfer_info).collect(),
        watches: state.watches.clone(),
        status: status_name(&state.remote.status).to_owned(),
        remote_revision: state.remote_revision,
    }
}

fn map_remote_directory(snapshot: &SftpSnapshot) -> SftpDirectorySummary {
    SftpDirectorySummary {
        path: snapshot.path.clone(),
        entries: snapshot
            .entries
            .iter()
            .map(|entry| SftpEntrySummary {
                name: entry.name.clone(),
                path: entry.path.clone(),
                is_directory: entry.is_directory,
                size: entry.size,
                modified_at: entry.modified_at.map(u64::from),
            })
            .collect(),
        loading: snapshot.loading,
        error: snapshot.error.clone(),
    }
}

fn map_local_directory(snapshot: &LocalSnapshot) -> SftpDirectorySummary {
    SftpDirectorySummary {
        path: snapshot.path.display().to_string(),
        entries: snapshot
            .entries
            .iter()
            .map(|entry| SftpEntrySummary {
                name: entry.name.clone(),
                path: entry.path.display().to_string(),
                is_directory: entry.is_directory,
                size: entry.size,
                modified_at: entry.modified_at.and_then(|modified_at| {
                    modified_at
                        .duration_since(UNIX_EPOCH)
                        .ok()
                        .map(|duration| duration.as_secs())
                }),
            })
            .collect(),
        loading: snapshot.loading,
        error: snapshot.error.clone(),
    }
}

fn map_transfer_info(transfer: &TransferRecord) -> SftpTransferInfo {
    let (source, is_directory) = match &transfer.request {
        TransferRequest::Upload {
            local_path,
            is_directory,
            ..
        } => (local_path.display().to_string(), *is_directory),
        TransferRequest::Download {
            remote_path,
            is_directory,
            ..
        } => (remote_path.clone(), *is_directory),
    };
    SftpTransferInfo {
        id: transfer.id,
        workspace_id: transfer.request.workspace_id().to_owned(),
        name: transfer.name.clone(),
        direction: transfer.direction.clone(),
        source,
        target: transfer.target.clone(),
        is_directory,
        progress: transfer.progress,
        transferred_bytes: transfer.transferred_bytes,
        total_bytes: transfer.total_bytes,
        speed_bytes_per_second: transfer.speed,
        status: transfer.status.clone(),
        error: transfer.error.clone(),
    }
}

fn status_name(status: &TerminalStatus) -> &'static str {
    match status {
        TerminalStatus::Connecting => "connecting",
        TerminalStatus::Connected => "connected",
        TerminalStatus::Disconnected => "disconnected",
        TerminalStatus::Failed => "failed",
    }
}

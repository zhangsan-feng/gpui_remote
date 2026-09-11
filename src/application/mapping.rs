use crate::domain::{
    session::{Protocol, SessionProfile},
    terminal::TerminalStatus,
};

use super::{
    ApplicationContext, LocalSnapshot, LocalWatchSummary, SftpSnapshot, SftpStatus, TransferRecord,
    TransferRequest,
    model::{
        SftpDirectorySummary, SftpEntrySummary, SftpTransferInfo, SftpTransferSummary,
        SftpWatchSummary, TerminalSummary,
    },
};

pub(crate) fn list_terminals(application: &ApplicationContext) -> Vec<TerminalSummary> {
    let selected_id = application.sessions().selected_id();
    application
        .sessions()
        .list()
        .into_iter()
        .filter(|(_, profile)| profile.protocol == Protocol::Ssh)
        .map(|(workspace_id, profile)| {
            let status = application
                .ssh()
                .snapshot(&workspace_id)
                .map(|data| terminal_status_name(&data.status).to_owned())
                .unwrap_or_else(|_| "connecting".to_owned());
            terminal_summary(workspace_id, profile, status, selected_id.as_deref())
        })
        .collect()
}

pub(crate) fn list_sftp_sessions(application: &ApplicationContext) -> Vec<TerminalSummary> {
    let selected_id = application.sessions().selected_id();
    application
        .sessions()
        .list()
        .into_iter()
        .filter(|(_, profile)| profile.protocol == Protocol::Sftp)
        .map(|(workspace_id, profile)| {
            let status = application
                .sftp()
                .snapshot(&workspace_id)
                .map(|snapshot| sftp_status_name(snapshot.status).to_owned())
                .unwrap_or_else(|_| "connecting".to_owned());
            terminal_summary(workspace_id, profile, status, selected_id.as_deref())
        })
        .collect()
}

pub(crate) fn map_remote_directory(snapshot: SftpSnapshot) -> SftpDirectorySummary {
    SftpDirectorySummary {
        path: snapshot.path,
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
        error: snapshot.error,
    }
}

pub(crate) fn map_local_directory(snapshot: LocalSnapshot) -> SftpDirectorySummary {
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
                        .duration_since(std::time::UNIX_EPOCH)
                        .ok()
                        .map(|duration| duration.as_secs())
                }),
            })
            .collect(),
        loading: snapshot.loading,
        error: snapshot.error,
    }
}

pub(crate) fn map_transfer_summary(transfers: Vec<TransferRecord>) -> SftpTransferSummary {
    SftpTransferSummary {
        queued: transfers.len(),
        transfers: transfers.into_iter().map(map_transfer_info).collect(),
    }
}

pub(crate) fn map_transfer_info(transfer: TransferRecord) -> SftpTransferInfo {
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
        name: transfer.name,
        direction: transfer.direction,
        source,
        target: transfer.target,
        is_directory,
        progress: transfer.progress,
        transferred_bytes: transfer.transferred_bytes,
        total_bytes: transfer.total_bytes,
        speed_bytes_per_second: transfer.speed,
        status: transfer.status,
        error: transfer.error,
    }
}

pub(crate) fn map_watch_summary(summary: LocalWatchSummary) -> SftpWatchSummary {
    SftpWatchSummary {
        workspace_id: summary.workspace_id,
        ip: summary.ip,
        title: summary.title,
        local_path: summary.local_path,
        remote_path: summary.remote_path,
        is_directory: summary.is_directory,
        debounce_ms: summary.debounce_ms,
    }
}

pub(crate) fn terminal_status_name(status: &TerminalStatus) -> &'static str {
    match status {
        TerminalStatus::Connecting => "connecting",
        TerminalStatus::Connected => "connected",
        TerminalStatus::Disconnected => "disconnected",
        TerminalStatus::Failed => "failed",
    }
}

pub(crate) fn sftp_status_name(status: SftpStatus) -> &'static str {
    match status {
        SftpStatus::Connecting => "connecting",
        SftpStatus::Connected => "connected",
        SftpStatus::Disconnected => "disconnected",
        SftpStatus::Failed => "failed",
    }
}

pub(crate) fn terminal_summary(
    workspace_id: String,
    profile: SessionProfile,
    status: String,
    selected_id: Option<&str>,
) -> TerminalSummary {
    TerminalSummary {
        workspace_id: workspace_id.clone(),
        profile_id: profile.id,
        ip: profile.host.clone(),
        title: profile.name.clone(),
        host: profile.host,
        protocol: profile.protocol.as_str().to_owned(),
        status,
        selected: selected_id == Some(workspace_id.as_str()),
    }
}

pub(crate) fn normalize_read_limit(limit: usize) -> usize {
    if limit == 0 { 200 } else { limit.min(2_000) }
}

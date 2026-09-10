use std::sync::Arc;

use crate::{
    application::{
        ApplicationContext, LocalSnapshot, LocalWatchSummary, RemoteDeleteItem, SftpSnapshot,
        SftpStatus, TransferRecord, TransferRequest,
    },
    domain::{
        session::{Protocol, SessionProfile},
        terminal::TerminalStatus,
    },
};

use super::{
    DataContextResult, SftpDirectorySummary, SftpEntrySummary, SftpTransferInfo,
    SftpTransferSummary, SftpWatchSummary, TerminalReadPage, TerminalSummary,
};

#[derive(Clone)]
pub struct GuiContext {
    application: ApplicationContext,
}

impl GuiContext {
    pub(crate) fn from_application(application: ApplicationContext) -> Self {
        Self { application }
    }

    pub(crate) fn terminal_updates(&self) -> Arc<tokio::sync::Notify> {
        self.application.ssh().updates()
    }

    pub(crate) fn terminal_status_updates(&self) -> Arc<tokio::sync::Notify> {
        self.application.ssh().status_updates()
    }

    pub(crate) fn sftp_updates(&self) -> Arc<tokio::sync::Notify> {
        self.application.sftp().updates()
    }

    pub(crate) fn sftp_status_updates(&self) -> Arc<tokio::sync::Notify> {
        self.application.sftp().status_updates()
    }

    pub(crate) fn terminal_snapshot(
        &self,
        workspace_id: &str,
    ) -> DataContextResult<crate::domain::terminal::TerminalData> {
        self.application.ssh().snapshot(workspace_id)
    }

    pub(crate) fn terminal_revision(&self, workspace_id: &str) -> DataContextResult<u64> {
        self.application.ssh().revision(workspace_id)
    }

    pub(crate) fn sftp_snapshot(
        &self,
        workspace_id: &str,
    ) -> DataContextResult<SftpDirectorySummary> {
        self.application
            .sftp()
            .snapshot(workspace_id)
            .map(map_remote_directory)
    }

    pub(crate) fn sftp_revision(&self, workspace_id: &str) -> DataContextResult<u64> {
        self.application.sftp().revision(workspace_id)
    }

    pub(crate) fn sftp_connection_status(
        &self,
        workspace_id: &str,
    ) -> DataContextResult<TerminalStatus> {
        self.application
            .sftp()
            .snapshot(workspace_id)
            .map(|snapshot| match snapshot.status {
                SftpStatus::Connecting => TerminalStatus::Connecting,
                SftpStatus::Connected => TerminalStatus::Connected,
                SftpStatus::Disconnected => TerminalStatus::Disconnected,
                SftpStatus::Failed => TerminalStatus::Failed,
            })
    }

    pub(crate) fn sftp_local_snapshot(
        &self,
        workspace_id: &str,
    ) -> DataContextResult<SftpDirectorySummary> {
        self.application
            .sftp()
            .local_snapshot(workspace_id)
            .map(map_local_directory)
    }

    pub(crate) fn sftp_transfers_snapshot(
        &self,
        workspace_id: &str,
    ) -> DataContextResult<Vec<SftpTransferInfo>> {
        self.application
            .sftp()
            .transfers(workspace_id)
            .map(|transfers| transfers.into_iter().map(map_transfer_info).collect())
    }

    pub async fn open_session(
        &self,
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
    ) -> DataContextResult<String> {
        self.application
            .open_session(profile_id, protocol, ip, title)
            .await
    }

    pub(crate) async fn close_session(&self, workspace_id: String) -> DataContextResult<()> {
        self.application.close_session(&workspace_id).await
    }

    pub(crate) async fn select_session(
        &self,
        workspace_id: Option<String>,
    ) -> DataContextResult<()> {
        self.application.select_session(workspace_id).await
    }

    pub(crate) async fn persist_sftp_local_path(
        &self,
        workspace_id: String,
        path: std::path::PathBuf,
    ) -> DataContextResult<()> {
        self.application
            .update_sftp_local_path(&workspace_id, &path)
            .await
    }

    pub(crate) async fn persist_sftp_remote_path(
        &self,
        workspace_id: String,
        path: String,
    ) -> DataContextResult<()> {
        self.application
            .update_sftp_remote_path(&workspace_id, path)
            .await
    }

    pub(crate) async fn delete_sftp_local_paths(
        &self,
        workspace_id: String,
        paths: Vec<std::path::PathBuf>,
    ) -> DataContextResult<()> {
        self.application
            .sftp()
            .delete_local_paths(&workspace_id, paths)
            .await
            .map(|_| ())
    }

    pub(crate) async fn delete_sftp_remote(
        &self,
        workspace_id: String,
        items: Vec<RemoteDeleteItem>,
    ) -> DataContextResult<()> {
        self.application
            .sftp()
            .delete_remote(&workspace_id, items)
            .await
    }

    pub async fn list_sftp_local(&self) -> DataContextResult<SftpDirectorySummary> {
        let workspace_id = selected_sftp_workspace(&self.application)?;
        self.application
            .sftp()
            .list_local(&workspace_id)
            .await
            .map(map_local_directory)
    }

    pub async fn list_sftp_sessions(&self) -> DataContextResult<Vec<TerminalSummary>> {
        Ok(list_sftp_sessions(&self.application))
    }

    pub async fn change_sftp_local_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> DataContextResult<()> {
        validate_session(
            &self.application,
            &workspace_id,
            Protocol::Sftp,
            &ip,
            &title,
        )?;
        self.application
            .sftp()
            .change_local_directory(&workspace_id, path.into())
            .await
            .map(|_| ())
    }

    pub async fn list_sftp_remote(
        &self,
        workspace_id: String,
    ) -> DataContextResult<SftpDirectorySummary> {
        self.sftp_snapshot(&workspace_id)
    }

    pub async fn change_sftp_remote_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> DataContextResult<()> {
        validate_session(
            &self.application,
            &workspace_id,
            Protocol::Sftp,
            &ip,
            &title,
        )?;
        self.application
            .sftp()
            .load_directory(&workspace_id, path)
            .await
    }

    pub async fn upload_sftp(
        &self,
        workspace_id: String,
        local_paths: Vec<String>,
    ) -> DataContextResult<SftpTransferSummary> {
        self.application
            .sftp()
            .upload(&workspace_id, local_paths)
            .await
            .map(map_transfer_summary)
    }

    pub async fn download_sftp(
        &self,
        workspace_id: String,
        remote_paths: Vec<String>,
    ) -> DataContextResult<SftpTransferSummary> {
        self.application
            .sftp()
            .download(&workspace_id, remote_paths)
            .await
            .map(map_transfer_summary)
    }

    pub(crate) async fn cancel_sftp_transfer(
        &self,
        workspace_id: String,
        transfer_id: u64,
    ) -> DataContextResult<()> {
        self.application
            .sftp()
            .cancel_transfer(&workspace_id, transfer_id)
    }

    pub(crate) async fn retry_sftp_transfer(
        &self,
        workspace_id: String,
        transfer_id: u64,
    ) -> DataContextResult<()> {
        self.application
            .sftp()
            .retry_transfer(&workspace_id, transfer_id)
            .await
    }

    pub async fn list_sftp_transfers(
        &self,
        workspace_id: String,
    ) -> DataContextResult<Vec<SftpTransferInfo>> {
        self.application
            .sftp()
            .transfers(&workspace_id)
            .map(|transfers| transfers.into_iter().map(map_transfer_info).collect())
    }

    pub async fn watch_sftp_local(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> DataContextResult<SftpWatchSummary> {
        validate_session(
            &self.application,
            &workspace_id,
            Protocol::Sftp,
            &ip,
            &title,
        )?;
        self.application
            .sftp()
            .watch_local(&workspace_id, local_path.into())
            .await
            .map(map_watch_summary)
    }

    pub async fn stop_sftp_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> DataContextResult<()> {
        validate_session(
            &self.application,
            &workspace_id,
            Protocol::Sftp,
            &ip,
            &title,
        )?;
        self.application
            .sftp()
            .stop_watching_local(&workspace_id, local_path.as_ref())
            .await
    }

    pub(crate) fn list_sftp_local_watches(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> DataContextResult<Vec<SftpWatchSummary>> {
        validate_session(
            &self.application,
            &workspace_id,
            Protocol::Sftp,
            &ip,
            &title,
        )?;
        self.application
            .sftp()
            .local_watches(&workspace_id)
            .map(|watches| watches.into_iter().map(map_watch_summary).collect())
    }

    pub async fn list_terminals(&self) -> DataContextResult<Vec<TerminalSummary>> {
        Ok(list_terminals(&self.application))
    }

    pub async fn select_terminal(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> DataContextResult<()> {
        validate_session(&self.application, &workspace_id, Protocol::Ssh, &ip, &title)?;
        self.application.select_session(Some(workspace_id)).await
    }

    pub async fn read_terminal(
        &self,
        workspace_id: Option<String>,
        offset: usize,
        limit: usize,
    ) -> DataContextResult<TerminalReadPage> {
        let workspace_id = resolve_terminal_id(&self.application, workspace_id)?;
        self.application
            .ssh()
            .read(&workspace_id, offset, normalize_read_limit(limit))
            .await
            .map(|page| TerminalReadPage {
                workspace_id,
                text: page.text,
                total_lines: page.total_lines,
                offset: page.offset,
                limit: page.limit,
                has_more: page.has_more,
            })
    }

    pub async fn send_text(
        &self,
        workspace_id: Option<String>,
        text: String,
    ) -> DataContextResult<()> {
        let workspace_id = resolve_terminal_id(&self.application, workspace_id)?;
        self.application
            .ssh()
            .send_input(&workspace_id, text.into_bytes())
            .await
    }

    pub async fn send_key(
        &self,
        workspace_id: Option<String>,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
    ) -> DataContextResult<()> {
        let workspace_id = resolve_terminal_id(&self.application, workspace_id)?;
        self.application
            .ssh()
            .send_key(&workspace_id, &key, control, alt, shift)
            .await
    }

    pub(crate) async fn send_terminal_input(
        &self,
        workspace_id: String,
        input: Vec<u8>,
    ) -> DataContextResult<()> {
        self.application
            .ssh()
            .send_input(&workspace_id, input)
            .await
    }

    pub(crate) async fn resize_terminal(
        &self,
        workspace_id: String,
        columns: u32,
        rows: u32,
    ) -> DataContextResult<()> {
        self.application
            .ssh()
            .resize(&workspace_id, columns, rows)
            .await
    }

    pub(crate) async fn scroll_terminal(
        &self,
        workspace_id: String,
        lines: i32,
    ) -> DataContextResult<()> {
        self.application.ssh().scroll(&workspace_id, lines).await
    }

    pub(crate) async fn scroll_terminal_to(
        &self,
        workspace_id: String,
        offset: usize,
    ) -> DataContextResult<()> {
        self.application
            .ssh()
            .scroll_to(&workspace_id, offset)
            .await
    }
}

fn selected_sftp_workspace(application: &ApplicationContext) -> DataContextResult<String> {
    let workspace_id = application
        .sessions()
        .selected_id()
        .ok_or_else(|| "当前没有选中的 SFTP 会话".to_owned())?;
    validate_session_protocol(application, &workspace_id, Protocol::Sftp)?;
    Ok(workspace_id)
}

fn resolve_terminal_id(
    application: &ApplicationContext,
    workspace_id: Option<String>,
) -> DataContextResult<String> {
    let workspace_id = workspace_id
        .or_else(|| application.sessions().selected_id())
        .ok_or_else(|| "当前没有选中的终端会话".to_owned())?;
    validate_session_protocol(application, &workspace_id, Protocol::Ssh)?;
    application
        .ssh()
        .snapshot(&workspace_id)
        .map(|_| workspace_id)
}

fn validate_session(
    application: &ApplicationContext,
    workspace_id: &str,
    protocol: Protocol,
    ip: &str,
    title: &str,
) -> DataContextResult<()> {
    let profile = application
        .sessions()
        .get(workspace_id)
        .ok_or_else(|| format!("会话不存在: {workspace_id}"))?;
    if profile.protocol != protocol {
        return Err(format!("会话协议不是 {protocol}: {workspace_id}"));
    }
    if profile.host != ip || profile.name != title {
        return Err(format!(
            "会话信息不匹配: {workspace_id}，请确认 ip 和 title"
        ));
    }
    Ok(())
}

fn validate_session_protocol(
    application: &ApplicationContext,
    workspace_id: &str,
    protocol: Protocol,
) -> DataContextResult<()> {
    let profile = application
        .sessions()
        .get(workspace_id)
        .ok_or_else(|| format!("会话不存在: {workspace_id}"))?;
    if profile.protocol != protocol {
        return Err(format!("会话协议不是 {protocol}: {workspace_id}"));
    }
    Ok(())
}

fn list_terminals(application: &ApplicationContext) -> Vec<TerminalSummary> {
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

fn list_sftp_sessions(application: &ApplicationContext) -> Vec<TerminalSummary> {
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

fn terminal_summary(
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

fn map_remote_directory(snapshot: SftpSnapshot) -> SftpDirectorySummary {
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

fn map_local_directory(snapshot: LocalSnapshot) -> SftpDirectorySummary {
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

fn map_transfer_summary(transfers: Vec<TransferRecord>) -> SftpTransferSummary {
    SftpTransferSummary {
        queued: transfers.len(),
        transfers: transfers.into_iter().map(map_transfer_info).collect(),
    }
}

fn map_transfer_info(transfer: TransferRecord) -> SftpTransferInfo {
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

fn map_watch_summary(summary: LocalWatchSummary) -> SftpWatchSummary {
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

fn terminal_status_name(status: &TerminalStatus) -> &'static str {
    match status {
        TerminalStatus::Connecting => "connecting",
        TerminalStatus::Connected => "connected",
        TerminalStatus::Disconnected => "disconnected",
        TerminalStatus::Failed => "failed",
    }
}

fn sftp_status_name(status: SftpStatus) -> &'static str {
    match status {
        SftpStatus::Connecting => "connecting",
        SftpStatus::Connected => "connected",
        SftpStatus::Disconnected => "disconnected",
        SftpStatus::Failed => "failed",
    }
}

fn normalize_read_limit(limit: usize) -> usize {
    if limit == 0 { 200 } else { limit.min(2_000) }
}

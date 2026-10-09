use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

use super::types::{
    ApplicationCommand, ApplicationResponse, COMMAND_CAPACITY, CommandEnvelope, McpBridgeEndpoint,
    McpBridgeReceiver, NOTIFICATION_CAPACITY, NotificationEnvelope,
};
use crate::{
    application::{
        ApplicationResult,
        mcp::{McpTerminalReadPage, TerminalSummary},
        session::model::{ProfileSummary, SshTunnelWorkspaceSummary},
    },
    data_context::{
        DATA_CONTEXT, SftpDirectorySummary, SftpTransferInfo, SftpTransferSummary,
        SftpWatchSummary, SftpWorkspaceSnapshot, WorkspaceSummary,
    },
    domain::{session::Protocol, terminal::TerminalStatus},
};

pub(crate) fn new() -> (McpBridgeEndpoint, McpBridgeReceiver) {
    let (command_tx, command_rx) = mpsc::channel(COMMAND_CAPACITY);
    let (notification_tx, _) = broadcast::channel(NOTIFICATION_CAPACITY);
    let data_context = &*DATA_CONTEXT;
    (
        McpBridgeEndpoint {
            command_tx,
            notification_tx: notification_tx.clone(),
            data_context,
        },
        McpBridgeReceiver {
            command_rx,
            notification_tx,
            data_context,
            notice: DATA_CONTEXT.notice.clone(),
        },
    )
}

impl McpBridgeEndpoint {
    pub(crate) async fn request(
        &self,
        command: ApplicationCommand,
    ) -> ApplicationResult<ApplicationResponse> {
        let request_id = Uuid::new_v4().to_string();
        let command_name = command.name();
        let workspace_id = command.workspace_id().unwrap_or("control").to_owned();
        let queued_at = std::time::Instant::now();
        log::debug!(
            "MCP bridge request started: request_id={request_id}, command={command_name}, workspace_id={workspace_id}"
        );
        let (response_tx, response_rx) = oneshot::channel();
        match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.command_tx.send(CommandEnvelope {
                request_id: request_id.clone(),
                command,
                response_tx,
                queued_at,
            }),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(_)) => {
                log::warn!(
                    "MCP bridge ingress closed: request_id={request_id}, command={command_name}, workspace_id={workspace_id}"
                );
                return Err("MCP application bridge 已关闭".to_owned());
            }
            Err(_) => {
                let queue_ms = queued_at.elapsed().as_millis();
                log::warn!(
                    "MCP bridge ingress queue full: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, queue_ms={queue_ms}"
                );
                return Err("MCP application bridge 队列已满".to_owned());
            }
        }
        log::debug!(
            "MCP bridge request enqueued: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, enqueue_ms={}",
            queued_at.elapsed().as_millis()
        );
        let response = response_rx
            .await
            .map_err(|_| "MCP application bridge 未返回响应".to_owned())?;
        log::debug!(
            "MCP bridge request response received: request_id={request_id}, command={command_name}, workspace_id={workspace_id}, total_ms={}",
            queued_at.elapsed().as_millis()
        );
        if response.request_id != request_id {
            return Err(format!(
                "MCP application bridge 响应关联 ID 不匹配: expected={request_id}, actual={}",
                response.request_id
            ));
        }
        response.result
    }

    pub(crate) fn subscribe_notifications(&self) -> broadcast::Receiver<NotificationEnvelope> {
        self.notification_tx.subscribe()
    }

    pub(crate) async fn read_profile_summaries(&self) -> ApplicationResult<Vec<ProfileSummary>> {
        match self.request(ApplicationCommand::ListProfiles).await? {
            ApplicationResponse::Profiles(profiles) => Ok(profiles),
            _ => Err("MCP application bridge 返回了错误的 profiles 响应".to_owned()),
        }
    }

    pub(crate) async fn open_session(
        &self,
        profile_id: String,
        protocol: Protocol,
        ip: String,
        title: String,
    ) -> ApplicationResult<String> {
        match self
            .request(ApplicationCommand::OpenSession {
                profile_id,
                protocol,
                ip,
                title,
            })
            .await?
        {
            ApplicationResponse::WorkspaceId(workspace_id) => Ok(workspace_id),
            _ => Err("MCP application bridge 返回了错误的 open_session 响应".to_owned()),
        }
    }

    pub(crate) async fn open_ssh_tunnel(
        &self,
        profile_id: String,
    ) -> ApplicationResult<SshTunnelWorkspaceSummary> {
        match self
            .request(ApplicationCommand::OpenSshTunnel { profile_id })
            .await?
        {
            ApplicationResponse::SshTunnelWorkspace(summary) => Ok(summary),
            _ => Err("MCP application bridge 返回了错误的 open_ssh_tunnel 响应".to_owned()),
        }
    }

    pub(crate) async fn read_sftp_workspace_summaries(
        &self,
    ) -> ApplicationResult<Vec<TerminalSummary>> {
        let snapshot = self.data_context.workspace_snapshot();
        let selected_id = snapshot.selected_workspace_id;
        Ok(snapshot
            .workspaces
            .into_iter()
            .filter(|workspace| workspace.protocol == Protocol::Sftp)
            .map(|workspace| terminal_summary(workspace, selected_id.as_deref()))
            .collect())
    }

    pub(crate) async fn close_session(&self, workspace_id: String) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::CloseSession { workspace_id })
            .await
    }

    pub(crate) async fn read_sftp_local_directory(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<SftpDirectorySummary> {
        self.sftp_snapshot(&workspace_id)
            .map(|snapshot| snapshot.local)
    }

    pub(crate) async fn change_sftp_local_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::ChangeSftpLocalDirectory {
            workspace_id,
            ip,
            title,
            path,
        })
        .await
    }

    pub(crate) async fn read_sftp_remote_directory(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<SftpDirectorySummary> {
        self.sftp_snapshot(&workspace_id)
            .map(|snapshot| snapshot.remote)
    }

    pub(crate) async fn change_sftp_remote_directory(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        path: String,
    ) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::ChangeSftpRemoteDirectory {
            workspace_id,
            ip,
            title,
            path,
        })
        .await
    }

    pub(crate) async fn upload_sftp(
        &self,
        workspace_id: String,
        local_paths: Vec<String>,
    ) -> ApplicationResult<SftpTransferSummary> {
        match self
            .request(ApplicationCommand::UploadSftp {
                workspace_id,
                local_paths,
            })
            .await?
        {
            ApplicationResponse::SftpTransferSummary(transfer) => Ok(transfer),
            _ => Err("MCP application bridge 返回了错误的上传响应".to_owned()),
        }
    }

    pub(crate) async fn download_sftp(
        &self,
        workspace_id: String,
        remote_paths: Vec<String>,
    ) -> ApplicationResult<SftpTransferSummary> {
        match self
            .request(ApplicationCommand::DownloadSftp {
                workspace_id,
                remote_paths,
            })
            .await?
        {
            ApplicationResponse::SftpTransferSummary(transfer) => Ok(transfer),
            _ => Err("MCP application bridge 返回了错误的下载响应".to_owned()),
        }
    }

    pub(crate) async fn read_sftp_transfer_records(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<Vec<SftpTransferInfo>> {
        self.sftp_snapshot(&workspace_id)
            .map(|snapshot| snapshot.transfers)
    }

    pub(crate) async fn start_sftp_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> ApplicationResult<SftpWatchSummary> {
        match self
            .request(ApplicationCommand::WatchSftpLocal {
                workspace_id,
                ip,
                title,
                local_path,
            })
            .await?
        {
            ApplicationResponse::SftpWatch(watch) => Ok(watch),
            _ => Err("MCP application bridge 返回了错误的 watch 响应".to_owned()),
        }
    }

    pub(crate) async fn stop_sftp_local_watch(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
        local_path: String,
    ) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::StopSftpLocalWatch {
            workspace_id,
            ip,
            title,
            local_path,
        })
        .await
    }

    pub(crate) async fn read_sftp_local_watch_summaries(
        &self,
        workspace_id: String,
        ip: String,
        title: String,
    ) -> ApplicationResult<Vec<SftpWatchSummary>> {
        let workspace = self
            .data_context
            .workspace_summary(&workspace_id)
            .ok_or_else(|| format!("会话不存在: {workspace_id}"))?;
        if workspace.protocol != Protocol::Sftp {
            return Err(format!("会话协议不是 SFTP: {workspace_id}"));
        }
        if workspace.host != ip || workspace.title != title {
            return Err(format!(
                "会话信息不匹配: {workspace_id}，请确认 ip 和 title"
            ));
        }
        self.data_context
            .sftp_workspace_snapshot(&workspace_id)
            .map(|snapshot| snapshot.watches)
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))
    }

    pub(crate) async fn list_terminals(&self) -> ApplicationResult<Vec<TerminalSummary>> {
        let snapshot = self.data_context.workspace_snapshot();
        let selected_id = snapshot.selected_workspace_id;
        Ok(snapshot
            .workspaces
            .into_iter()
            .filter(|workspace| workspace.protocol == Protocol::Ssh)
            .map(|workspace| terminal_summary(workspace, selected_id.as_deref()))
            .collect())
    }

    pub(crate) async fn mcp_read_terminal(
        &self,
        workspace_id: String,
        offset: usize,
        limit: usize,
        since_mcp_snapshot_version: Option<u64>,
    ) -> ApplicationResult<McpTerminalReadPage> {
        let page = self
            .data_context
            .terminal_history(
                &workspace_id,
                offset,
                normalize_read_limit(limit),
                since_mcp_snapshot_version,
            )
            .ok_or_else(|| format!("SSH 终端历史不存在: {workspace_id}"))?;
        Ok(McpTerminalReadPage {
            workspace_id,
            text: page.text,
            total_lines: page.total_lines,
            offset: page.offset,
            limit: page.limit,
            has_more: page.has_more,
            mcp_snapshot_version: page.mcp_snapshot_version,
            changed: page.changed,
        })
    }

    pub(crate) async fn send_text(
        &self,
        workspace_id: String,
        text: String,
    ) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::SendText { workspace_id, text })
            .await
    }

    pub(crate) async fn send_key(
        &self,
        workspace_id: String,
        key: String,
        control: bool,
        alt: bool,
        shift: bool,
    ) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::SendKey {
            workspace_id,
            key,
            control,
            alt,
            shift,
        })
        .await
    }

    async fn expect_empty(&self, command: ApplicationCommand) -> ApplicationResult<()> {
        match self.request(command).await? {
            ApplicationResponse::Empty => Ok(()),
            _ => Err("MCP application bridge 返回了错误的操作响应".to_owned()),
        }
    }

    fn sftp_snapshot(&self, workspace_id: &str) -> ApplicationResult<SftpWorkspaceSnapshot> {
        self.data_context
            .sftp_workspace_snapshot(workspace_id)
            .ok_or_else(|| format!("SFTP 会话不存在: {workspace_id}"))
    }
}

fn normalize_read_limit(limit: usize) -> usize {
    if limit == 0 { 200 } else { limit.min(2_000) }
}

fn terminal_summary(workspace: WorkspaceSummary, selected_id: Option<&str>) -> TerminalSummary {
    let selected = selected_id == Some(workspace.workspace_id.as_str());
    TerminalSummary {
        workspace_id: workspace.workspace_id,
        profile_id: workspace.profile_id,
        ip: workspace.host.clone(),
        title: workspace.title,
        host: workspace.host,
        protocol: workspace.protocol.as_str().to_owned(),
        status: match workspace.status {
            TerminalStatus::Connecting => "connecting",
            TerminalStatus::Connected => "connected",
            TerminalStatus::Disconnected => "disconnected",
            TerminalStatus::Failed => "failed",
        }
        .to_owned(),
        selected,
    }
}

use tokio::sync::{broadcast, mpsc, oneshot};
use uuid::Uuid;

use super::types::{
    ApplicationCommand, ApplicationResponse, COMMAND_CAPACITY, CommandEnvelope, McpBridgeEndpoint,
    McpBridgeReceiver, NOTIFICATION_CAPACITY, NotificationEnvelope,
};
use crate::{
    application::{
        ApplicationResult,
        model::{
            ProfileSummary, SftpDirectorySummary, SftpTransferInfo, SftpTransferSummary,
            SftpWatchSummary, TerminalReadPage, TerminalSummary,
        },
    },
    domain::session::Protocol,
};

pub(crate) fn new() -> (McpBridgeEndpoint, McpBridgeReceiver) {
    let (command_tx, command_rx) = mpsc::channel(COMMAND_CAPACITY);
    let (notification_tx, _) = broadcast::channel(NOTIFICATION_CAPACITY);
    (
        McpBridgeEndpoint {
            command_tx,
            notification_tx: notification_tx.clone(),
        },
        McpBridgeReceiver {
            command_rx,
            notification_tx,
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

    pub(crate) async fn read_sftp_workspace_summaries(
        &self,
    ) -> ApplicationResult<Vec<TerminalSummary>> {
        match self.request(ApplicationCommand::ListSftpSessions).await? {
            ApplicationResponse::TerminalSummaries(sessions) => Ok(sessions),
            _ => Err("MCP application bridge 返回了错误的 SFTP 会话响应".to_owned()),
        }
    }

    pub(crate) async fn close_session(&self, workspace_id: String) -> ApplicationResult<()> {
        self.expect_empty(ApplicationCommand::CloseSession { workspace_id })
            .await
    }

    pub(crate) async fn read_sftp_local_directory(
        &self,
        workspace_id: String,
    ) -> ApplicationResult<SftpDirectorySummary> {
        match self
            .request(ApplicationCommand::ListSftpLocal { workspace_id })
            .await?
        {
            ApplicationResponse::SftpDirectory(directory) => Ok(directory),
            _ => Err("MCP application bridge 返回了错误的本地目录响应".to_owned()),
        }
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
        match self
            .request(ApplicationCommand::ListSftpRemote { workspace_id })
            .await?
        {
            ApplicationResponse::SftpDirectory(directory) => Ok(directory),
            _ => Err("MCP application bridge 返回了错误的远程目录响应".to_owned()),
        }
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
        match self
            .request(ApplicationCommand::ListSftpTransfers { workspace_id })
            .await?
        {
            ApplicationResponse::SftpTransferInfos(transfers) => Ok(transfers),
            _ => Err("MCP application bridge 返回了错误的传输响应".to_owned()),
        }
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
        match self
            .request(ApplicationCommand::ListSftpLocalWatches {
                workspace_id,
                ip,
                title,
            })
            .await?
        {
            ApplicationResponse::SftpWatches(watches) => Ok(watches),
            _ => Err("MCP application bridge 返回了错误的 watches 响应".to_owned()),
        }
    }

    pub(crate) async fn list_terminals(&self) -> ApplicationResult<Vec<TerminalSummary>> {
        match self.request(ApplicationCommand::ListTerminals).await? {
            ApplicationResponse::TerminalSummaries(terminals) => Ok(terminals),
            _ => Err("MCP application bridge 返回了错误的终端响应".to_owned()),
        }
    }

    pub(crate) async fn read_terminal(
        &self,
        workspace_id: String,
        offset: usize,
        limit: usize,
        since_revision: Option<u64>,
    ) -> ApplicationResult<TerminalReadPage> {
        match self
            .request(ApplicationCommand::ReadTerminal {
                workspace_id,
                offset,
                limit,
                since_revision,
            })
            .await?
        {
            ApplicationResponse::TerminalRead(page) => Ok(page),
            _ => Err("MCP application bridge 返回了错误的终端读取响应".to_owned()),
        }
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
}

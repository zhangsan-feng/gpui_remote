use rmcp::{
    ErrorData, Json,
    handler::server::wrapper::Parameters,
    schemars::{self, JsonSchema},
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::model::{
        ProfileSummary, SftpDirectorySummary, SftpEntrySummary, SftpTransferInfo,
        SftpTransferSummary, SftpWatchSummary, TerminalReadPage, TerminalSummary,
    },
    domain::session::Protocol,
};

use super::bridge::McpBridgeEndpoint;

#[derive(Clone)]
pub(super) struct AgentTerminalMcp {
    bridge: McpBridgeEndpoint,
}

impl AgentTerminalMcp {
    pub(super) fn new(bridge: McpBridgeEndpoint) -> Self {
        Self { bridge }
    }
}

#[derive(Deserialize, JsonSchema)]
struct OpenSessionInput {
    profile_id: String,
    ip: String,
    title: String,
    protocol: OpenSessionProtocol,
}

#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum OpenSessionProtocol {
    Ssh,
    Sftp,
}

#[derive(Deserialize, JsonSchema)]
struct ReadTerminalInput {
    workspace_id: String,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_read_limit")]
    limit: usize,
}

#[derive(Deserialize, JsonSchema)]
struct SftpWorkspaceInput {
    workspace_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct SftpChangeDirectoryInput {
    workspace_id: String,
    ip: String,
    title: String,
    path: String,
}

#[derive(Deserialize, JsonSchema)]
struct SftpUploadInput {
    workspace_id: String,
    local_paths: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
struct SftpDownloadInput {
    workspace_id: String,
    remote_paths: Vec<String>,
}

#[derive(Deserialize, JsonSchema)]
struct SftpWatchPathInput {
    workspace_id: String,
    ip: String,
    title: String,
    local_path: String,
}

#[derive(Deserialize, JsonSchema)]
struct SftpWatchListInput {
    workspace_id: String,
    ip: String,
    title: String,
}

#[derive(Deserialize, JsonSchema)]
struct SendTextInput {
    workspace_id: String,
    text: String,
}

#[derive(Deserialize, JsonSchema)]
struct SendKeyInput {
    workspace_id: String,
    key: String,
    #[serde(default)]
    control: bool,
    #[serde(default)]
    alt: bool,
    #[serde(default)]
    shift: bool,
}

#[derive(Serialize, JsonSchema)]
struct ProfileOutput {
    id: String,
    title: String,
    ip: String,
    host: String,
    protocol: String,
}

#[derive(Serialize, JsonSchema)]
struct OpenSessionOutput {
    workspace_id: String,
    ip: String,
    title: String,
}

#[derive(Serialize, JsonSchema)]
struct TerminalOutput {
    workspace_id: String,
    profile_id: String,
    ip: String,
    title: String,
    host: String,
    protocol: String,
    status: String,
    selected: bool,
}

#[derive(Serialize, JsonSchema)]
struct TerminalReadOutput {
    workspace_id: String,
    text: String,
    total_lines: usize,
    offset: usize,
    limit: usize,
    has_more: bool,
}

#[derive(Serialize, JsonSchema)]
struct SftpEntryOutput {
    name: String,
    path: String,
    is_directory: bool,
    size: u64,
}

#[derive(Serialize, JsonSchema)]
struct SftpDirectoryOutput {
    path: String,
    entries: Vec<SftpEntryOutput>,
    loading: bool,
    error: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct SftpTransferOutput {
    queued: usize,
    transfers: Vec<SftpTransferInfoOutput>,
}

#[derive(Serialize, JsonSchema)]
struct SftpTransferInfoOutput {
    id: u64,
    workspace_id: String,
    name: String,
    direction: String,
    source: String,
    target: String,
    is_directory: bool,
    progress: f32,
    transferred_bytes: u64,
    total_bytes: u64,
    speed_bytes_per_second: u64,
    status: String,
    error: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct SftpWatchOutput {
    workspace_id: String,
    ip: String,
    title: String,
    local_path: String,
    remote_path: String,
    is_directory: bool,
    debounce_ms: u64,
}

#[derive(Serialize, JsonSchema)]
struct ActionOutput {
    success: bool,
}

#[tool_router]
impl AgentTerminalMcp {
    #[tool(
        description = "List saved connection profiles. Returns profile id, title, ip, host, and protocol."
    )]
    async fn list_profiles(&self) -> Result<Json<Vec<ProfileOutput>>, ErrorData> {
        let started_at = std::time::Instant::now();
        log::debug!("MCP tool handler started: tool=list_profiles");
        let result = self
            .bridge
            .read_profile_summaries()
            .await
            .map(|profiles| Json(profiles.into_iter().map(ProfileOutput::from).collect()))
            .map_err(mcp_error);
        log::debug!(
            "MCP tool handler finished: tool=list_profiles, ok={}, elapsed_ms={}",
            result.is_ok(),
            started_at.elapsed().as_millis()
        );
        result
    }

    #[tool(
        description = "Open a new workspace top_session using a saved profile id, ip, title, and the requested protocol. The ip and title must match the saved profile."
    )]
    async fn open_session(
        &self,
        Parameters(input): Parameters<OpenSessionInput>,
    ) -> Result<Json<OpenSessionOutput>, ErrorData> {
        let ip = input.ip.clone();
        let title = input.title.clone();
        self.bridge
            .open_session(
                input.profile_id,
                input.protocol.into(),
                input.ip,
                input.title,
            )
            .await
            .map(|workspace_id| {
                Json(OpenSessionOutput {
                    workspace_id,
                    ip,
                    title,
                })
            })
            .map_err(mcp_error)
    }

    #[tool(description = "Close an open SSH or SFTP workspace by workspace id.")]
    async fn close_session(
        &self,
        Parameters(input): Parameters<SftpWorkspaceInput>,
    ) -> Result<Json<ActionOutput>, ErrorData> {
        self.bridge
            .close_session(input.workspace_id)
            .await
            .map(|()| Json(ActionOutput { success: true }))
            .map_err(mcp_error)
    }

    #[tool(description = "List open SFTP sessions by workspace id.")]
    async fn list_sftp_sessions(&self) -> Result<Json<Vec<TerminalOutput>>, ErrorData> {
        self.bridge
            .read_sftp_workspace_summaries()
            .await
            .map(|sessions| Json(sessions.into_iter().map(TerminalOutput::from).collect()))
            .map_err(mcp_error)
    }

    #[tool(description = "List the local directory currently shown by an open SFTP workspace.")]
    async fn list_sftp_local(
        &self,
        Parameters(input): Parameters<SftpWorkspaceInput>,
    ) -> Result<Json<SftpDirectoryOutput>, ErrorData> {
        self.bridge
            .read_sftp_local_directory(input.workspace_id)
            .await
            .map(|directory| Json(directory.into()))
            .map_err(mcp_error)
    }

    #[tool(
        description = "Change the local directory shown by the workspace_id SFTP session; ip and title must match the SFTP session."
    )]
    async fn change_sftp_local_directory(
        &self,
        Parameters(input): Parameters<SftpChangeDirectoryInput>,
    ) -> Result<Json<ActionOutput>, ErrorData> {
        self.bridge
            .change_sftp_local_directory(input.workspace_id, input.ip, input.title, input.path)
            .await
            .map(|()| Json(ActionOutput { success: true }))
            .map_err(mcp_error)
    }

    #[tool(description = "List the current remote directory for an open SFTP workspace.")]
    async fn list_sftp_remote(
        &self,
        Parameters(input): Parameters<SftpWorkspaceInput>,
    ) -> Result<Json<SftpDirectoryOutput>, ErrorData> {
        self.bridge
            .read_sftp_remote_directory(input.workspace_id)
            .await
            .map(|directory| Json(directory.into()))
            .map_err(mcp_error)
    }

    #[tool(
        description = "Change the remote directory shown by an open SFTP workspace. ip and title must match the SFTP session; the directory path is resolved by the remote SFTP server."
    )]
    async fn change_sftp_remote_directory(
        &self,
        Parameters(input): Parameters<SftpChangeDirectoryInput>,
    ) -> Result<Json<ActionOutput>, ErrorData> {
        self.bridge
            .change_sftp_remote_directory(input.workspace_id, input.ip, input.title, input.path)
            .await
            .map(|()| Json(ActionOutput { success: true }))
            .map_err(mcp_error)
    }

    #[tool(
        description = "Queue one or more local files or directories for upload to the identified workspace's current remote SFTP directory."
    )]
    async fn upload_sftp(
        &self,
        Parameters(input): Parameters<SftpUploadInput>,
    ) -> Result<Json<SftpTransferOutput>, ErrorData> {
        self.bridge
            .upload_sftp(input.workspace_id, input.local_paths)
            .await
            .map(|transfer| Json(transfer.into()))
            .map_err(mcp_error)
    }

    #[tool(
        description = "Queue one or more entries from the identified workspace's current remote SFTP directory for download to its current local directory."
    )]
    async fn download_sftp(
        &self,
        Parameters(input): Parameters<SftpDownloadInput>,
    ) -> Result<Json<SftpTransferOutput>, ErrorData> {
        self.bridge
            .download_sftp(input.workspace_id, input.remote_paths)
            .await
            .map(|transfer| Json(transfer.into()))
            .map_err(mcp_error)
    }

    #[tool(
        description = "List complete upload and download details for an SFTP workspace, including source, target, status, progress, byte counts, speed, and errors."
    )]
    async fn list_sftp_transfers(
        &self,
        Parameters(input): Parameters<SftpWorkspaceInput>,
    ) -> Result<Json<Vec<SftpTransferInfoOutput>>, ErrorData> {
        self.bridge
            .read_sftp_transfer_records(input.workspace_id)
            .await
            .map(|transfers| Json(transfers.into_iter().map(Into::into).collect()))
            .map_err(mcp_error)
    }

    #[tool(
        description = "Watch a local SFTP file or directory in the current session. Changes are uploaded after a fixed 2-second debounce; the watch is not persisted."
    )]
    async fn watch_sftp_local(
        &self,
        Parameters(input): Parameters<SftpWatchPathInput>,
    ) -> Result<Json<SftpWatchOutput>, ErrorData> {
        self.bridge
            .start_sftp_local_watch(input.workspace_id, input.ip, input.title, input.local_path)
            .await
            .map(|watch| Json(watch.into()))
            .map_err(mcp_error)
    }

    #[tool(description = "Stop watching a local SFTP file or directory in the current session.")]
    async fn stop_sftp_local_watch(
        &self,
        Parameters(input): Parameters<SftpWatchPathInput>,
    ) -> Result<Json<ActionOutput>, ErrorData> {
        self.bridge
            .stop_sftp_local_watch(input.workspace_id, input.ip, input.title, input.local_path)
            .await
            .map(|()| Json(ActionOutput { success: true }))
            .map_err(mcp_error)
    }

    #[tool(description = "List local SFTP files and directories watched in the current session.")]
    async fn list_sftp_local_watches(
        &self,
        Parameters(input): Parameters<SftpWatchListInput>,
    ) -> Result<Json<Vec<SftpWatchOutput>>, ErrorData> {
        self.bridge
            .read_sftp_local_watch_summaries(input.workspace_id, input.ip, input.title)
            .await
            .map(|watches| Json(watches.into_iter().map(Into::into).collect()))
            .map_err(mcp_error)
    }

    #[tool(description = "List open SSH terminal sessions by workspace id.")]
    async fn list_terminals(&self) -> Result<Json<Vec<TerminalOutput>>, ErrorData> {
        self.bridge
            .list_terminals()
            .await
            .map(|terminals| Json(terminals.into_iter().map(TerminalOutput::from).collect()))
            .map_err(mcp_error)
    }

    #[tool(
        description = "Read terminal output for an SSH workspace without changing GUI scroll position. Offset counts lines back from the newest output."
    )]
    async fn read_terminal(
        &self,
        Parameters(input): Parameters<ReadTerminalInput>,
    ) -> Result<Json<TerminalReadOutput>, ErrorData> {
        self.bridge
            .read_terminal(input.workspace_id, input.offset, input.limit)
            .await
            .map(|page| Json(TerminalReadOutput::from(page)))
            .map_err(mcp_error)
    }

    #[tool(description = "Send UTF-8 text to an SSH workspace terminal.")]
    async fn send_text(
        &self,
        Parameters(input): Parameters<SendTextInput>,
    ) -> Result<Json<ActionOutput>, ErrorData> {
        self.bridge
            .send_text(input.workspace_id, input.text)
            .await
            .map(|()| Json(ActionOutput { success: true }))
            .map_err(mcp_error)
    }

    #[tool(
        description = "Send a terminal key. Named keys include enter, tab, escape, arrows, home, end, delete, pageup, pagedown, insert, and f1-f12."
    )]
    async fn send_key(
        &self,
        Parameters(input): Parameters<SendKeyInput>,
    ) -> Result<Json<ActionOutput>, ErrorData> {
        self.bridge
            .send_key(
                input.workspace_id,
                input.key,
                input.control,
                input.alt,
                input.shift,
            )
            .await
            .map(|()| Json(ActionOutput { success: true }))
            .map_err(mcp_error)
    }
}

#[tool_handler(
    instructions = "This MCP server uses single-request/single-final-result tool semantics. The HTTP transport may be Streamable HTTP, but each tool invocation returns one final result and does not stream incremental output. Long-running SFTP transfers are queued by upload_sftp or download_sftp; use list_sftp_transfers to query their later status and progress."
)]
impl rmcp::ServerHandler for AgentTerminalMcp {}

impl From<OpenSessionProtocol> for Protocol {
    fn from(protocol: OpenSessionProtocol) -> Self {
        match protocol {
            OpenSessionProtocol::Ssh => Self::Ssh,
            OpenSessionProtocol::Sftp => Self::Sftp,
        }
    }
}

impl From<ProfileSummary> for ProfileOutput {
    fn from(profile: ProfileSummary) -> Self {
        Self {
            id: profile.id,
            title: profile.title,
            ip: profile.host.clone(),
            host: profile.host,
            protocol: profile.protocol,
        }
    }
}

impl From<TerminalSummary> for TerminalOutput {
    fn from(terminal: TerminalSummary) -> Self {
        Self {
            workspace_id: terminal.workspace_id,
            profile_id: terminal.profile_id,
            ip: terminal.ip,
            title: terminal.title,
            host: terminal.host,
            protocol: terminal.protocol,
            status: terminal.status,
            selected: terminal.selected,
        }
    }
}

impl From<TerminalReadPage> for TerminalReadOutput {
    fn from(page: TerminalReadPage) -> Self {
        Self {
            workspace_id: page.workspace_id,
            text: page.text,
            total_lines: page.total_lines,
            offset: page.offset,
            limit: page.limit,
            has_more: page.has_more,
        }
    }
}

impl From<SftpDirectorySummary> for SftpDirectoryOutput {
    fn from(directory: SftpDirectorySummary) -> Self {
        Self {
            path: directory.path,
            entries: directory.entries.into_iter().map(Into::into).collect(),
            loading: directory.loading,
            error: directory.error,
        }
    }
}

impl From<SftpEntrySummary> for SftpEntryOutput {
    fn from(entry: SftpEntrySummary) -> Self {
        Self {
            name: entry.name,
            path: entry.path,
            is_directory: entry.is_directory,
            size: entry.size,
        }
    }
}

impl From<SftpTransferSummary> for SftpTransferOutput {
    fn from(transfer: SftpTransferSummary) -> Self {
        Self {
            queued: transfer.queued,
            transfers: transfer.transfers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SftpTransferInfo> for SftpTransferInfoOutput {
    fn from(transfer: SftpTransferInfo) -> Self {
        Self {
            id: transfer.id,
            workspace_id: transfer.workspace_id,
            name: transfer.name,
            direction: transfer.direction,
            source: transfer.source,
            target: transfer.target,
            is_directory: transfer.is_directory,
            progress: transfer.progress,
            transferred_bytes: transfer.transferred_bytes,
            total_bytes: transfer.total_bytes,
            speed_bytes_per_second: transfer.speed_bytes_per_second,
            status: transfer.status,
            error: transfer.error,
        }
    }
}

impl From<SftpWatchSummary> for SftpWatchOutput {
    fn from(watch: SftpWatchSummary) -> Self {
        Self {
            workspace_id: watch.workspace_id,
            ip: watch.ip,
            title: watch.title,
            local_path: watch.local_path,
            remote_path: watch.remote_path,
            is_directory: watch.is_directory,
            debounce_ms: watch.debounce_ms,
        }
    }
}

fn default_read_limit() -> usize {
    200
}

fn mcp_error(message: String) -> ErrorData {
    ErrorData::internal_error(message, None)
}

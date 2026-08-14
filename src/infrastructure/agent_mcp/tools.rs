use rmcp::{
    ErrorData, Json,
    handler::server::wrapper::Parameters,
    schemars::{self, JsonSchema},
    tool, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::agent_mcp::{
        AgentMcpClient, ProfileSummary, SftpDirectorySummary, SftpEntrySummary, SftpTransferInfo,
        SftpTransferSummary, TerminalReadPage, TerminalSummary,
    },
    domain::session::Protocol,
};

#[derive(Clone)]
pub(super) struct AgentTerminalMcp {
    client: AgentMcpClient,
}

impl AgentTerminalMcp {
    pub(super) fn new(client: AgentMcpClient) -> Self {
        Self { client }
    }
}

#[derive(Deserialize, JsonSchema)]
struct OpenSessionInput {
    profile_id: String,
    protocol: OpenSessionProtocol,
}

#[derive(Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum OpenSessionProtocol {
    Ssh,
    Sftp,
}

#[derive(Deserialize, JsonSchema)]
struct SelectTerminalInput {
    workspace_id: String,
}

#[derive(Deserialize, JsonSchema)]
struct ReadTerminalInput {
    workspace_id: Option<String>,
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
struct SendTextInput {
    workspace_id: Option<String>,
    text: String,
}

#[derive(Deserialize, JsonSchema)]
struct SendKeyInput {
    workspace_id: Option<String>,
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
    host: String,
}

#[derive(Serialize, JsonSchema)]
struct OpenSessionOutput {
    workspace_id: String,
}

#[derive(Serialize, JsonSchema)]
struct TerminalOutput {
    workspace_id: String,
    profile_id: String,
    host: String,
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
struct ActionOutput {
    success: bool,
}

#[tool_router(server_handler)]
impl AgentTerminalMcp {
    #[tool(description = "List saved connection profiles. Returns only profile id and host.")]
    async fn list_profiles(&self) -> Result<Json<Vec<ProfileOutput>>, ErrorData> {
        self.client
            .list_profiles()
            .await
            .map(|profiles| Json(profiles.into_iter().map(ProfileOutput::from).collect()))
            .map_err(mcp_error)
    }

    #[tool(
        description = "Open a new workspace top_session using a saved profile id and the requested protocol."
    )]
    async fn open_session(
        &self,
        Parameters(input): Parameters<OpenSessionInput>,
    ) -> Result<Json<OpenSessionOutput>, ErrorData> {
        self.client
            .open_session(input.profile_id, input.protocol.into())
            .await
            .map(|workspace_id| Json(OpenSessionOutput { workspace_id }))
            .map_err(mcp_error)
    }

    #[tool(description = "List the local directory currently shown by the SFTP workspace.")]
    async fn list_sftp_local(&self) -> Result<Json<SftpDirectoryOutput>, ErrorData> {
        self.client
            .list_sftp_local()
            .await
            .map(|directory| Json(directory.into()))
            .map_err(mcp_error)
    }

    #[tool(description = "List the current remote directory for an open SFTP workspace.")]
    async fn list_sftp_remote(
        &self,
        Parameters(input): Parameters<SftpWorkspaceInput>,
    ) -> Result<Json<SftpDirectoryOutput>, ErrorData> {
        self.client
            .list_sftp_remote(input.workspace_id)
            .await
            .map(|directory| Json(directory.into()))
            .map_err(mcp_error)
    }

    #[tool(
        description = "Queue one or more local files or directories for upload to the current remote SFTP directory."
    )]
    async fn upload_sftp(
        &self,
        Parameters(input): Parameters<SftpUploadInput>,
    ) -> Result<Json<SftpTransferOutput>, ErrorData> {
        self.client
            .upload_sftp(input.workspace_id, input.local_paths)
            .await
            .map(|transfer| Json(transfer.into()))
            .map_err(mcp_error)
    }

    #[tool(
        description = "Queue one or more entries from the current remote SFTP directory for download to the current local directory."
    )]
    async fn download_sftp(
        &self,
        Parameters(input): Parameters<SftpDownloadInput>,
    ) -> Result<Json<SftpTransferOutput>, ErrorData> {
        self.client
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
        self.client
            .list_sftp_transfers(input.workspace_id)
            .await
            .map(|transfers| Json(transfers.into_iter().map(Into::into).collect()))
            .map_err(mcp_error)
    }

    #[tool(description = "List open terminal sessions and identify the selected top_session.")]
    async fn list_terminals(&self) -> Result<Json<Vec<TerminalOutput>>, ErrorData> {
        self.client
            .list_terminals()
            .await
            .map(|terminals| Json(terminals.into_iter().map(TerminalOutput::from).collect()))
            .map_err(mcp_error)
    }

    #[tool(description = "Switch the GUI to an open terminal top_session.")]
    async fn select_terminal(
        &self,
        Parameters(input): Parameters<SelectTerminalInput>,
    ) -> Result<Json<ActionOutput>, ErrorData> {
        self.client
            .select_terminal(input.workspace_id)
            .await
            .map(|()| Json(ActionOutput { success: true }))
            .map_err(mcp_error)
    }

    #[tool(
        description = "Read terminal output without changing GUI scroll position. Offset counts lines back from the newest output."
    )]
    async fn read_terminal(
        &self,
        Parameters(input): Parameters<ReadTerminalInput>,
    ) -> Result<Json<TerminalReadOutput>, ErrorData> {
        self.client
            .read_terminal(input.workspace_id, input.offset, input.limit)
            .await
            .map(|page| Json(TerminalReadOutput::from(page)))
            .map_err(mcp_error)
    }

    #[tool(description = "Send UTF-8 text to a terminal top_session.")]
    async fn send_text(
        &self,
        Parameters(input): Parameters<SendTextInput>,
    ) -> Result<Json<ActionOutput>, ErrorData> {
        self.client
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
        self.client
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
            host: profile.host,
        }
    }
}

impl From<TerminalSummary> for TerminalOutput {
    fn from(terminal: TerminalSummary) -> Self {
        Self {
            workspace_id: terminal.workspace_id,
            profile_id: terminal.profile_id,
            host: terminal.host,
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

fn default_read_limit() -> usize {
    200
}

fn mcp_error(message: String) -> ErrorData {
    ErrorData::internal_error(message, None)
}

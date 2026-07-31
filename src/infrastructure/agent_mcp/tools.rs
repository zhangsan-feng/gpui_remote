use rmcp::{
    ErrorData, Json,
    handler::server::wrapper::Parameters,
    schemars::{self, JsonSchema},
    tool, tool_router,
};
use serde::{Deserialize, Serialize};

use crate::{
    application::agent_mcp::{AgentMcpClient, ProfileSummary, TerminalReadPage, TerminalSummary},
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
        description = "Open a new workspace session using a saved profile id and the requested protocol."
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

    #[tool(description = "List open terminal sessions and identify the selected session.")]
    async fn list_terminals(&self) -> Result<Json<Vec<TerminalOutput>>, ErrorData> {
        self.client
            .list_terminals()
            .await
            .map(|terminals| Json(terminals.into_iter().map(TerminalOutput::from).collect()))
            .map_err(mcp_error)
    }

    #[tool(description = "Switch the GUI to an open terminal session.")]
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

    #[tool(description = "Send UTF-8 text to a terminal session.")]
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

fn default_read_limit() -> usize {
    200
}

fn mcp_error(message: String) -> ErrorData {
    ErrorData::internal_error(message, None)
}

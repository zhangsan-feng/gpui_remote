use crate::{
    application::{ApplicationResult, session::validation::validate_session_protocol},
    data_context::DATA_CONTEXT,
    domain::session::Protocol,
};

pub(crate) fn validate_terminal_workspace(
    sessions: &crate::application::session::SessionApplication,
    workspace_id: &str,
) -> ApplicationResult<()> {
    validate_session_protocol(sessions, workspace_id, Protocol::Ssh)?;
    DATA_CONTEXT
        .terminal_snapshot(workspace_id)
        .map(|_| ())
        .ok_or_else(|| format!("SSH 会话不存在: {workspace_id}"))
}

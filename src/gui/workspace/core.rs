use std::collections::HashMap;

use crate::domain::terminal::TerminalStatus;

use super::{
    session::{OpenedWorkspaceSession, WorkspaceSession},
    terminal::TerminalView,
};

pub(super) fn tab_statuses(
    sessions: &[(OpenedWorkspaceSession, TerminalStatus)],
) -> HashMap<String, TerminalStatus> {
    sessions
        .iter()
        .map(|(opened_session, status)| (opened_session.session.id.clone(), status.clone()))
        .collect()
}

pub(super) fn workspace_state_data(
    workspace: &WorkspaceSession,
    terminal: &TerminalView,
) -> (
    Vec<(OpenedWorkspaceSession, TerminalStatus)>,
    Option<String>,
) {
    let workspace_sessions = workspace
        .sessions()
        .iter()
        .map(|opened_session| {
            let status = terminal
                .model(&opened_session.session.id)
                .map(|model| model.read().status.clone())
                .unwrap_or(TerminalStatus::Connecting);
            (opened_session.clone(), status)
        })
        .collect();
    (
        workspace_sessions,
        workspace.active_session_id().map(str::to_owned),
    )
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum DataContextEvent {
    WorkspaceChanged,
    TerminalChanged { workspace_id: String },
    SftpChanged { workspace_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplicationEvent {
    SessionOpened {
        workspace_id: String,
        profile: crate::domain::session::SessionProfile,
    },
    SessionClosed {
        workspace_id: String,
    },
    SessionSelected {
        workspace_id: Option<String>,
    },
}

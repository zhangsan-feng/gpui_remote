use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ProfileSummary {
    pub id: String,
    pub host: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TerminalSummary {
    pub workspace_id: String,
    pub profile_id: String,
    pub host: String,
    pub status: String,
    pub selected: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct TerminalReadPage {
    pub workspace_id: String,
    pub text: String,
    pub total_lines: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
}

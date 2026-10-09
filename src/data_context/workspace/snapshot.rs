use super::super::{DataContext, DataSnapshot, WorkspaceSummary};

impl DataContext {
    pub(crate) fn workspace_snapshot(&self) -> DataSnapshot {
        self.snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub(crate) fn workspace_summary(&self, workspace_id: &str) -> Option<WorkspaceSummary> {
        self.snapshot
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .workspaces
            .iter()
            .find(|workspace| workspace.workspace_id == workspace_id)
            .cloned()
    }
}

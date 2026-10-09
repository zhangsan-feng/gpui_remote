use gpui_kit::*;

use crate::{
    data_context::{DATA_CONTEXT, DataContext, DataSnapshot},
    domain::session::Protocol,
};

use super::SftpView;

impl SftpView {
    pub(super) fn refresh_from_context(&mut self, _cx: &mut Context<Self>) {
        let context = &*DATA_CONTEXT;
        let workspace_ids = self.projections.keys().cloned().collect::<Vec<_>>();
        for workspace_id in workspace_ids {
            self.refresh_workspace_from_context(&context, &workspace_id);
        }
    }

    fn refresh_workspace_from_context(&mut self, context: &DataContext, workspace_id: &str) {
        let snapshot = match context.sftp_workspace_snapshot(workspace_id) {
            Some(snapshot) => snapshot,
            None => {
                log::debug!("SFTP GUI projection refresh skipped: workspace_id={workspace_id}");
                return;
            }
        };
        if self.selected_workspace_id.as_deref() == Some(workspace_id) {
            self.local = snapshot.local.clone();
            self.transfers = snapshot.transfers.clone();
        }
        if let Some(projection) = self.projections.get_mut(workspace_id) {
            projection.snapshot = snapshot;
        }
    }

    pub(in crate::gui::workspace) fn apply_snapshot(
        &mut self,
        snapshot: &DataSnapshot,
        cx: &mut Context<Self>,
    ) {
        let existing = self.projections.keys().cloned().collect::<Vec<_>>();
        for workspace_id in existing {
            if !snapshot.workspaces.iter().any(|workspace| {
                workspace.workspace_id == workspace_id && workspace.protocol == Protocol::Sftp
            }) {
                self.close(&workspace_id);
            }
        }
        let context = &*DATA_CONTEXT;
        let mut refresh_workspaces = Vec::new();
        for workspace in snapshot
            .workspaces
            .iter()
            .filter(|workspace| workspace.protocol == Protocol::Sftp)
        {
            let is_new = !self.projections.contains_key(&workspace.workspace_id);
            if is_new {
                self.initialize_projection(workspace.workspace_id.clone(), workspace.clone());
            }
            let previous_revision = self
                .observed_sftp_revisions
                .insert(workspace.workspace_id.clone(), workspace.sftp_revision);
            if is_new || previous_revision != Some(workspace.sftp_revision) {
                refresh_workspaces.push(workspace.workspace_id.as_str());
            }
        }
        let selected = snapshot
            .selected_workspace_id
            .as_ref()
            .filter(|workspace_id| self.projections.contains_key(workspace_id.as_str()));
        let selection_changed = self.selected_workspace_id != selected.cloned();
        if selection_changed {
            if let Some(previous_workspace_id) = self.selected_workspace_id.as_deref() {
                self.local_restore_requests.remove(previous_workspace_id);
            }
            self.selected_workspace_id = selected.cloned();
            self.remote_list_state.reset_with_uniform_height(0, px(38.));
            if let Some(workspace_id) = selected {
                self.restore_local_path(workspace_id, cx);
            }
        }
        let has_refresh = !refresh_workspaces.is_empty();
        for workspace_id in refresh_workspaces {
            self.refresh_workspace_from_context(&context, workspace_id);
        }
        if selection_changed || has_refresh {
            cx.notify();
        }
    }
}

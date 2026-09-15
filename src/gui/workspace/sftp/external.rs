use std::sync::Arc;

use gpui_kit::*;

use crate::{
    application::ApplicationContext,
    domain::session::Protocol,
    global_state::{GlobalEvent, read_global_state},
};

use super::SftpView;

impl SftpView {
    pub(super) fn refresh_from_application(&mut self, cx: &mut Context<Self>) {
        let application =
            cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
        let workspace_ids = self.projections.keys().cloned().collect::<Vec<_>>();
        let selected_workspace_id = self.selected_workspace_id.clone();
        for workspace_id in workspace_ids {
            let snapshot = match application.sftp_workspace_snapshot(&workspace_id) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    log::debug!(
                        "SFTP GUI projection refresh skipped: workspace_id={workspace_id}, error={error}"
                    );
                    continue;
                }
            };
            if selected_workspace_id.as_deref() == Some(workspace_id.as_str()) {
                self.local = snapshot.local.clone();
                self.transfers = snapshot.transfers.clone();
            }
            if let Some(projection) = self.projections.get_mut(&workspace_id) {
                projection.snapshot = snapshot;
            }
        }
    }

    pub(super) fn start_subscribe(&self, cx: &mut Context<Self>) {
        let global_state = read_global_state(cx);
        cx.subscribe(&global_state, |this, _, event, cx| {
            match event {
                GlobalEvent::WorkspaceSessionOpened(workspace_id, profile)
                    if profile.protocol == Protocol::Sftp =>
                {
                    log::debug!(
                        "SFTP GUI projection event received: workspace_id={workspace_id}, profile_id={}, host={}",
                        profile.id,
                        profile.host
                    );
                    this.initialize_projection(workspace_id.clone(), profile.clone());
                }
                GlobalEvent::WorkspaceSessionSelected(workspace_id) => {
                    if this.selected_workspace_id == *workspace_id {
                        return;
                    }
                    if let Some(previous_workspace_id) = this.selected_workspace_id.as_deref() {
                        this.local_restore_requests.remove(previous_workspace_id);
                    }
                    this.selected_workspace_id = workspace_id.clone();
                    this.remote_list_state.reset_with_uniform_height(0, px(38.));
                    if let Some(workspace_id) = workspace_id
                        .as_deref()
                        .filter(|workspace_id| this.projections.contains_key(*workspace_id))
                    {
                        this.restore_local_path(workspace_id, cx);
                    }
                    this.refresh_from_application(cx);
                }
                GlobalEvent::WorkspaceSessionClosed { workspace_id } => {
                    this.close(workspace_id);
                }
                _ => return,
            }
            cx.notify();
        })
        .detach();
    }

    pub(in crate::gui::workspace) fn status_updates(&self) -> Arc<tokio::sync::Notify> {
        self.status_updates.clone()
    }
}

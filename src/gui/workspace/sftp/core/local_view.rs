use std::path::PathBuf;

use gpui_kit::*;

use super::super::SftpView;

impl SftpView {
    pub(in crate::gui::workspace::sftp) fn change_local_directory(
        &mut self,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.navigate_local_directory(path, true, cx);
    }

    pub(in crate::gui::workspace::sftp) fn change_local_directory_without_history(
        &mut self,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.navigate_local_directory(path, false, cx);
    }

    // Parent, back, double-click and dialog navigation all use this entry point.
    fn navigate_local_directory(
        &mut self,
        path: PathBuf,
        record_history: bool,
        cx: &mut Context<Self>,
    ) {
        let current_path = PathBuf::from(&self.local.path);
        let path_changed = PathBuf::from(&self.local.path) != path;
        if record_history && path_changed && !self.local.path.is_empty() {
            if let Some(workspace_id) = self.selected_workspace_id.clone() {
                self.push_local_back_path(&workspace_id, current_path);
            }
        }
        self.change_local_directory_inner(path, cx);
    }

    pub(in crate::gui::workspace::sftp) fn can_go_local_back(&self) -> bool {
        self.selected_workspace_id
            .as_deref()
            .and_then(|workspace_id| self.local_back_history.get(workspace_id))
            .is_some_and(|history| !history.is_empty())
    }

    pub(in crate::gui::workspace::sftp) fn pop_local_back_path(&mut self) -> Option<PathBuf> {
        let workspace_id = self.selected_workspace_id.as_deref()?;
        self.local_back_history
            .get_mut(workspace_id)
            .and_then(Vec::pop)
    }

    fn push_local_back_path(&mut self, workspace_id: &str, path: PathBuf) {
        let history = self
            .local_back_history
            .entry(workspace_id.to_owned())
            .or_default();
        if history.last() != Some(&path) {
            history.push(path);
        }
    }

    fn change_local_directory_inner(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        let Some(projection) = self.projections.get_mut(&workspace_id) else {
            return;
        };
        let profile_ip = projection.profile_ip.clone();
        let profile_title = projection.profile_title.clone();
        let previous = projection.local_navigation_tail.take();
        let (complete, next) = tokio::sync::oneshot::channel();
        projection.local_navigation_tail = Some(next);
        self.local_selection.clear();
        self.local.path = path.display().to_string();
        self.local.loading = true;
        self.local.error = None;
        self.local_list_state.reset_with_uniform_height(0, px(38.));
        cx.notify();

        let application = crate::application::APPLICATION.clone();
        let path_text = path.display().to_string();
        cx.spawn(async move |this, cx| {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            let task_workspace_id = workspace_id.clone();
            let result = crate::global_state::run_application(async move {
                application
                    .sftp
                    .change_local_directory_checked(
                        task_workspace_id,
                        profile_ip,
                        profile_title,
                        path_text,
                    )
                    .await
            })
            .await;
            let _ = complete.send(());
            let _ = this.update(cx, |this, cx| {
                if this.selected_workspace_id.as_deref() != Some(workspace_id.as_str()) {
                    return;
                }
                if let Err(error) = result {
                    this.local.loading = false;
                    this.local.error = Some(error);
                }
                this.refresh_from_context(cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::gui::workspace::sftp) fn restore_local_path(
        &mut self,
        workspace_id: &str,
        cx: &mut Context<Self>,
    ) {
        if !claim_local_restore(&mut self.local_restore_requests, workspace_id) {
            log::debug!("SFTP 本地目录恢复请求已处理，跳过重复恢复: workspace={workspace_id}");
            return;
        }
        self.refresh_from_context(cx);
        self.local_selection.clear();
        self.local_list_state.reset_with_uniform_height(0, px(38.));
        self.local_restore_requests.remove(workspace_id);
    }
}

fn claim_local_restore(
    pending: &mut std::collections::HashSet<String>,
    workspace_id: &str,
) -> bool {
    pending.insert(workspace_id.to_owned())
}

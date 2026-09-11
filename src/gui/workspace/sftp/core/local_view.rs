use std::path::PathBuf;

use gpui_kit::*;

use super::super::SftpView;

impl SftpView {
    pub(in crate::gui::workspace::sftp) fn load_local_directory(
        &mut self,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.load_local_directory_inner(path, cx);
    }

    fn load_local_directory_inner(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        let Some(projection) = self.projections.get(&workspace_id) else {
            return;
        };
        let profile_ip = projection.profile_ip.clone();
        let profile_title = projection.profile_title.clone();
        self.local_selection.clear();
        self.local.path = path.clone();
        self.local.loading = true;
        self.local.error = None;
        self.local_list_state.reset_with_uniform_height(0, px(38.));
        cx.notify();
        log::debug!(
            "SFTP 本地目录扫描开始: workspace={}, path={}",
            workspace_id,
            path.display()
        );

        let application = self.application.clone();
        let path_text = path.display().to_string();
        cx.spawn(async move |this, cx| {
            let result = application
                .change_sftp_local_directory(
                    workspace_id.clone(),
                    profile_ip,
                    profile_title,
                    path_text,
                )
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.selected_workspace_id.as_deref() != Some(workspace_id.as_str()) {
                    return;
                }
                if let Err(error) = result {
                    this.local.loading = false;
                    this.local.error = Some(error);
                }
                this.sync_application_state();
                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::gui::workspace::sftp) fn persist_local_path_for_selected_workspace(
        &self,
        path: &std::path::Path,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_id) = self.selected_workspace_id.as_deref() else {
            return;
        };
        self.persist_local_directory(workspace_id, path, cx);
    }

    pub(in crate::gui::workspace::sftp) fn restore_local_path(
        &mut self,
        workspace_id: &str,
        _cx: &mut Context<Self>,
    ) {
        if !claim_local_restore(&mut self.local_restore_requests, workspace_id) {
            log::debug!("SFTP 本地目录恢复请求已处理，跳过重复恢复: workspace={workspace_id}");
            return;
        }
        self.sync_application_state();
        self.local_selection.clear();
        self.local_list_state.reset_with_uniform_height(0, px(38.));
        self.local_restore_requests.remove(workspace_id);
    }

    fn persist_local_directory(
        &self,
        workspace_id: &str,
        path: &std::path::Path,
        cx: &mut Context<Self>,
    ) {
        let Some(profile_id) = self
            .projections
            .get(workspace_id)
            .map(|projection| projection.profile_id.clone())
        else {
            return;
        };
        let application = self.application.clone();
        let workspace_id = workspace_id.to_owned();
        let path = path.to_owned();
        log::debug!(
            "SFTP 本地目录路径变化，准备保存: session={}, path={}",
            profile_id,
            path.display()
        );
        cx.spawn(async move |_this, _cx| {
            match application
                .persist_sftp_local_path(workspace_id, path)
                .await
            {
                Ok(()) => log::debug!("SFTP 本地目录保存完成: session={profile_id}"),
                Err(error) => log::warn!("保存 SFTP 本地目录失败，会话 {profile_id}: {error}"),
            }
        })
        .detach();
    }
}

fn claim_local_restore(
    pending: &mut std::collections::HashSet<String>,
    workspace_id: &str,
) -> bool {
    pending.insert(workspace_id.to_owned())
}

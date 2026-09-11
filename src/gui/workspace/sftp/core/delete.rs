use gpui_kit::*;

use crate::application::ApplicationContext;

use super::super::{DeleteLocalEntry, DeleteRemoteEntry, SftpView};

impl SftpView {
    pub(in crate::gui::workspace::sftp) fn delete_local_entry(
        &mut self,
        action: &DeleteLocalEntry,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if action.0.is_empty() {
            return;
        }
        let paths = action.0.clone();
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        let current_directory = std::path::PathBuf::from(&self.local.path);
        self.local_selection.clear();
        self.local.loading = true;
        self.local.error = None;
        log::debug!(
            "SFTP 批量删除本地路径开始: directory={}, count={}",
            current_directory.display(),
            paths.len()
        );
        cx.notify();

        let application =
            cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
        cx.spawn(async move |this, cx| {
            let result = application
                .delete_sftp_local_paths(workspace_id, paths)
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.local.path != current_directory.display().to_string() {
                    return;
                }
                this.local.loading = false;
                if let Err(error) = result {
                    this.local.error = Some(error);
                }
                this.refresh_from_application(cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::gui::workspace::sftp) fn delete_remote_entry(
        &mut self,
        action: &DeleteRemoteEntry,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if action.items.is_empty() {
            return;
        }
        let Some(snapshot) = self.selected_snapshot() else {
            return;
        };
        let Some(workspace_id) = self.selected_workspace_id.as_deref() else {
            return;
        };
        let items = action.items.clone();
        let count = items.len();
        let refresh_path = snapshot.remote.path;
        let application =
            cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
        self.remote_selection.clear();
        log::debug!(
            "SFTP 批量删除远程路径加入队列: workspace_id={}, count={}, directory={refresh_path}",
            workspace_id,
            count
        );
        let task_workspace_id = workspace_id.to_owned();
        cx.spawn(async move |_this, _cx| {
            let items = items
                .into_iter()
                .map(|item| crate::application::RemoteDeleteItem {
                    path: item.path,
                    is_directory: item.is_directory,
                })
                .collect();
            if let Err(error) = application
                .delete_sftp_remote_paths(task_workspace_id, items)
                .await
            {
                log::warn!("SFTP 批量删除请求失败: {error}");
            }
        })
        .detach();
        cx.notify();
    }
}

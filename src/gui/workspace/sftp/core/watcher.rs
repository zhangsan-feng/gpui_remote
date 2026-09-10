use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use gpui_kit::*;

use super::super::{SftpView, StopWatchingLocalPath, WatchLocalPath};

impl SftpView {
    pub(in crate::gui::workspace::sftp) fn is_local_path_watched(
        &self,
        workspace_id: &str,
        local_path: &Path,
    ) -> bool {
        self.local_watchers
            .get(workspace_id)
            .is_some_and(|watches| watches.contains_key(local_path))
    }

    pub(in crate::gui::workspace::sftp) fn local_watched_paths(&self) -> HashSet<PathBuf> {
        self.selected_workspace_id
            .as_deref()
            .and_then(|workspace_id| self.local_watchers.get(workspace_id))
            .map(|watches| watches.keys().cloned().collect())
            .unwrap_or_default()
    }

    pub(in crate::gui::workspace::sftp) fn watch_local_path(
        &mut self,
        action: &WatchLocalPath,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        self.watch_local_path_for_workspace(&workspace_id, action.0.clone(), cx);
    }

    fn watch_local_path_for_workspace(
        &mut self,
        workspace_id: &str,
        local_path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let Some(projection) = self.projections.get(workspace_id) else {
            return;
        };
        let profile_ip = projection.profile_ip.clone();
        let profile_title = projection.profile_title.clone();
        let gui = self.gui.clone();
        let task_workspace_id = workspace_id.to_owned();
        let local_path_text = local_path.display().to_string();
        cx.spawn(async move |this, cx| {
            match gui
                .watch_sftp_local(
                    task_workspace_id.clone(),
                    profile_ip,
                    profile_title,
                    local_path_text,
                )
                .await
            {
                Ok(summary) => {
                    let _ = this.update(cx, |this, cx| {
                        let local_path = PathBuf::from(&summary.local_path);
                        this.local_watchers
                            .entry(task_workspace_id)
                            .or_default()
                            .insert(local_path, summary);
                        cx.notify();
                    });
                }
                Err(error) => log::warn!("开启本地自动上传监听失败: {error}"),
            }
        })
        .detach();
    }

    pub(in crate::gui::workspace::sftp) fn stop_watching_local_path(
        &mut self,
        action: &StopWatchingLocalPath,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return;
        };
        let Some(projection) = self.projections.get(&workspace_id) else {
            return;
        };
        let gui = self.gui.clone();
        let profile_ip = projection.profile_ip.clone();
        let profile_title = projection.profile_title.clone();
        let local_path = action.0.clone();
        let local_path_text = local_path.display().to_string();
        cx.spawn(async move |this, cx| {
            match gui
                .stop_sftp_local_watch(
                    workspace_id.clone(),
                    profile_ip,
                    profile_title,
                    local_path_text,
                )
                .await
            {
                Ok(()) => {
                    let _ = this.update(cx, |this, cx| {
                        if let Some(watches) = this.local_watchers.get_mut(&workspace_id) {
                            watches.remove(&local_path);
                            if watches.is_empty() {
                                this.local_watchers.remove(&workspace_id);
                            }
                        }
                        cx.notify();
                    });
                }
                Err(error) => log::warn!("停止本地自动上传监听失败: {error}"),
            }
        })
        .detach();
    }

    pub(in crate::gui::workspace::sftp) fn stop_local_watchers_for_workspace(
        &mut self,
        workspace_id: &str,
    ) {
        self.local_watchers.remove(workspace_id);
    }

    pub(in crate::gui::workspace::sftp) fn stop_all_local_watchers(&mut self) {
        self.local_watchers.clear();
    }
}

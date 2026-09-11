use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use gpui_kit::*;

use crate::application::ApplicationContext;

use super::super::{SftpView, StopWatchingLocalPath, WatchLocalPath};

impl SftpView {
    pub(in crate::gui::workspace::sftp) fn is_local_path_watched(
        &self,
        workspace_id: &str,
        local_path: &Path,
    ) -> bool {
        self.projections
            .get(workspace_id)
            .is_some_and(|projection| {
                projection
                    .snapshot
                    .watches
                    .iter()
                    .any(|watch| Path::new(&watch.local_path) == local_path)
            })
    }

    pub(in crate::gui::workspace::sftp) fn local_watched_paths(&self) -> HashSet<PathBuf> {
        self.selected_workspace_id
            .as_deref()
            .and_then(|workspace_id| self.projections.get(workspace_id))
            .map(|projection| {
                projection
                    .snapshot
                    .watches
                    .iter()
                    .map(|watch| PathBuf::from(&watch.local_path))
                    .collect()
            })
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
        let application =
            cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
        let task_workspace_id = workspace_id.to_owned();
        let local_path_text = local_path.display().to_string();
        cx.spawn(async move |this, cx| {
            match application
                .start_sftp_local_watch(
                    task_workspace_id,
                    profile_ip,
                    profile_title,
                    local_path_text,
                )
                .await
            {
                Ok(_) => {
                    let _ = this.update(cx, |this, cx| {
                        this.refresh_from_application(cx);
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
        let application =
            cx.read_global::<ApplicationContext, _>(|application, _| application.clone());
        let profile_ip = projection.profile_ip.clone();
        let profile_title = projection.profile_title.clone();
        let local_path = action.0.clone();
        let local_path_text = local_path.display().to_string();
        cx.spawn(async move |this, cx| {
            match application
                .stop_sftp_local_watch(workspace_id, profile_ip, profile_title, local_path_text)
                .await
            {
                Ok(()) => {
                    let _ = this.update(cx, |this, cx| {
                        this.refresh_from_application(cx);
                        cx.notify();
                    });
                }
                Err(error) => log::warn!("停止本地自动上传监听失败: {error}"),
            }
        })
        .detach();
    }

    pub(in crate::gui::workspace::sftp) fn clear_local_watch_projection_for_workspace(
        &mut self,
        workspace_id: &str,
    ) {
        if let Some(projection) = self.projections.get_mut(workspace_id) {
            projection.snapshot.watches.clear();
        }
    }

    pub(in crate::gui::workspace::sftp) fn clear_all_local_watch_projections(&mut self) {
        for projection in self.projections.values_mut() {
            projection.snapshot.watches.clear();
        }
    }
}

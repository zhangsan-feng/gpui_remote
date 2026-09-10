use std::{collections::HashSet, path::PathBuf, sync::Arc};

use gpui_kit::*;

use crate::infrastructure::storage::Storage;

use super::super::SftpView;
use super::{default_desktop_path, local};

impl SftpView {
    pub(in crate::gui::workspace::sftp) fn load_local_directory(
        &mut self,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.load_local_directory_inner(path, cx);
    }

    fn load_local_directory_inner(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let load_workspace_id = self.selected_workspace_id.clone();
        self.local_selection.clear();
        self.local.path = path.clone();
        self.local.loading = true;
        self.local.error = None;
        self.local_list_state.reset_with_uniform_height(0, px(38.));
        cx.notify();
        log::debug!("SFTP 本地目录扫描开始: {}", path.display());

        cx.spawn(async move |this, cx| {
            let result = tokio::task::spawn_blocking(move || local::read_local_directory(&path))
                .await
                .map_err(|error| anyhow::anyhow!("读取本地目录任务失败: {error}"))
                .and_then(|result| result);
            let _ = this.update(cx, |this, cx| {
                if this.selected_workspace_id != load_workspace_id {
                    if let Some(workspace_id) = load_workspace_id.as_deref() {
                        this.local_restore_requests.remove(workspace_id);
                    }
                    return;
                }
                this.local.loading = false;
                match result {
                    Ok((path, entries)) => {
                        log::debug!(
                            "SFTP 本地目录扫描完成: {}, entries={}",
                            path.display(),
                            entries.len()
                        );
                        this.local.path = path.clone();
                        this.local.entries = Arc::new(entries);
                        this.local.error = None;
                    }
                    Err(error) => {
                        log::warn!("SFTP 本地目录扫描失败: {error:#}");
                        this.local.error = Some(format!("{error:#}"));
                    }
                }
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
        cx: &mut Context<Self>,
    ) {
        if !claim_local_restore(&mut self.local_restore_requests, workspace_id) {
            log::debug!("SFTP 本地目录恢复请求已处理，跳过重复恢复: workspace={workspace_id}");
            return;
        }
        let Some(profile_id) = self
            .runtimes
            .get(workspace_id)
            .map(|runtime| runtime.profile_id.clone())
        else {
            self.local_restore_requests.remove(workspace_id);
            return;
        };
        let session = cx.global::<Storage>().session.clone();
        let workspace_id = workspace_id.to_owned();
        cx.spawn(async move |this, cx| {
            let local_path = tokio::task::spawn_blocking(move || {
                let state = session.sftp_state(&profile_id)?;
                Ok::<_, anyhow::Error>(
                    state
                        .and_then(|state| state.local_path)
                        .unwrap_or_else(default_desktop_path),
                )
            })
            .await
            .map_err(|error| anyhow::anyhow!("读取 SFTP 本地目录任务失败: {error}"))
            .and_then(|result| result);
            let local_path = match local_path {
                Ok(path) => path,
                Err(error) => {
                    log::warn!("读取 SFTP 本地目录失败，会话 {workspace_id}: {error:#}");
                    tokio::task::spawn_blocking(default_desktop_path)
                        .await
                        .unwrap_or_else(|task_error| {
                            log::warn!("解析默认本地目录任务失败: {task_error}");
                            PathBuf::from(".")
                        })
                }
            };
            let _ = this.update(cx, |this, cx| {
                if this.selected_workspace_id.as_deref() != Some(workspace_id.as_str()) {
                    this.local_restore_requests.remove(&workspace_id);
                    return;
                }
                apply_restored_local_path(&mut this.local, local_path);
                this.local_selection.clear();
                this.local_list_state.reset_with_uniform_height(0, px(38.));
                log::debug!(
                    "SFTP 本地目录路径恢复完成，开始扫描: workspace={}, path={}",
                    workspace_id,
                    this.local.path.display()
                );
                this.load_local_directory(this.local.path.clone(), cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn persist_local_directory(
        &self,
        workspace_id: &str,
        path: &std::path::Path,
        cx: &mut Context<Self>,
    ) {
        let Some(profile_id) = self
            .runtimes
            .get(workspace_id)
            .map(|runtime| runtime.profile_id.clone())
        else {
            return;
        };
        let session = cx.global::<Storage>().session.clone();
        let path = path.to_owned();
        let task_profile_id = profile_id.clone();
        log::debug!(
            "SFTP 本地目录路径变化，准备保存: session={}, path={}",
            profile_id,
            path.display()
        );
        cx.spawn(async move |_this, _cx| {
            let result = tokio::task::spawn_blocking(move || {
                session.update_sftp_local_path(&task_profile_id, &path)
            })
            .await;
            match result {
                Ok(Ok(())) => {
                    log::debug!("SFTP 本地目录保存完成: session={profile_id}");
                }
                Ok(Err(error)) => {
                    log::warn!("保存 SFTP 本地目录失败，会话 {profile_id}: {error:#}");
                }
                Err(error) => {
                    log::warn!("保存 SFTP 本地目录任务失败，会话 {profile_id}: {error}");
                }
            }
        })
        .detach();
    }
}

fn apply_restored_local_path(snapshot: &mut super::super::LocalSnapshot, path: PathBuf) {
    snapshot.path = path;
    snapshot.entries = Arc::new(Vec::new());
    snapshot.loading = false;
    snapshot.error = None;
}

fn claim_local_restore(pending: &mut HashSet<String>, workspace_id: &str) -> bool {
    pending.insert(workspace_id.to_owned())
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, path::PathBuf, sync::Arc};

    use super::{apply_restored_local_path, claim_local_restore};

    #[test]
    fn restoring_local_path_clears_entries_without_loading_directory() {
        let mut snapshot = super::super::super::LocalSnapshot {
            path: PathBuf::from("old"),
            entries: Arc::new(Vec::new()),
            loading: true,
            error: Some("stale".to_owned()),
        };

        apply_restored_local_path(&mut snapshot, PathBuf::from("new"));

        assert_eq!(snapshot.path, PathBuf::from("new"));
        assert!(snapshot.entries.is_empty());
        assert!(!snapshot.loading);
        assert_eq!(snapshot.error, None);
    }

    #[test]
    fn only_one_local_restore_request_is_claimed_per_workspace() {
        let mut pending = HashSet::new();

        assert!(claim_local_restore(&mut pending, "workspace-1"));
        assert!(!claim_local_restore(&mut pending, "workspace-1"));
        assert!(claim_local_restore(&mut pending, "workspace-2"));
    }
}

use gpui_kit::*;

use super::super::{DeleteLocalEntry, DeleteRemoteEntry, SftpCommand, SftpView};
use super::local;

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
        let current_directory = self.local.path.clone();
        self.local_selection.clear();
        self.local.loading = true;
        self.local.error = None;
        log::debug!(
            "SFTP 批量删除本地路径开始: directory={}, count={}",
            current_directory.display(),
            paths.len()
        );
        cx.notify();

        let refresh_directory = current_directory.clone();
        cx.spawn(async move |this, cx| {
            let result = tokio::task::spawn_blocking(move || {
                let mut errors = Vec::new();
                for path in paths {
                    if let Err(error) = local::delete_local_path(&path) {
                        log::warn!("SFTP 删除本地路径失败: {}: {error:#}", path.display());
                        errors.push(format!("{}: {error:#}", path.display()));
                    }
                }
                let directory = local::read_local_directory(&refresh_directory)?;
                Ok::<_, anyhow::Error>((directory, errors))
            })
            .await
            .map_err(|error| anyhow::anyhow!("删除本地路径任务失败: {error}"))
            .and_then(|result| result);

            let _ = this.update(cx, |this, cx| {
                if this.local.path != current_directory {
                    return;
                }
                this.local.loading = false;
                match result {
                    Ok(((path, entries), errors)) => {
                        this.local.path = path;
                        this.local.entries = std::sync::Arc::new(entries);
                        this.local.error = (!errors.is_empty()).then(|| errors.join("\n"));
                        cx.notify();
                    }
                    Err(error) => {
                        this.local.error = Some(format!("{error:#}"));
                        cx.notify();
                    }
                }
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
        let Some(runtime) = self.runtimes.get(workspace_id) else {
            return;
        };
        let items = action.items.clone();
        let count = items.len();
        let refresh_path = snapshot.path;
        runtime.model.set_loading();
        self.remote_selection.clear();
        log::debug!(
            "SFTP 批量删除远程路径加入队列: workspace_id={}, count={}, directory={refresh_path}",
            workspace_id,
            count
        );
        if runtime
            .commands
            .send(SftpCommand::Delete {
                items,
                refresh_path,
            })
            .is_err()
        {
            runtime.model.set_error("SFTP 连接已关闭".to_owned());
        }
        cx.notify();
    }
}

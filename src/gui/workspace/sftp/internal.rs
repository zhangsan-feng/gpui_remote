use chrono::{DateTime, Local};
use gpui::*;
use std::{path::PathBuf, time::SystemTime};

use super::{SftpSnapshot, SftpView};

impl SftpView {
    pub(super) fn selected_snapshot(&self) -> Option<SftpSnapshot> {
        let workspace_id = self.selected_workspace_id.as_deref()?;
        Some(self.runtimes.get(workspace_id)?.model.snapshot())
    }

    pub(super) fn open_directory(&mut self, path: String, cx: &mut Context<Self>) {
        self.load_directory(path);
        cx.notify();
    }

    pub(super) fn open_local_directory(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.load_local_directory(path, cx);
    }

    pub(super) fn go_local_parent(
        &mut self,
        _: &ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(parent) = self.local.path.parent().map(PathBuf::from) else {
            return;
        };
        self.load_local_directory(parent, cx);
    }

    pub(super) fn refresh_local(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.load_local_directory(self.local.path.clone(), cx);
    }

    pub(super) fn go_parent(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.selected_snapshot() else {
            return;
        };
        self.load_directory(parent_path(&snapshot.path));
        cx.notify();
    }

    pub(super) fn refresh(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let Some(snapshot) = self.selected_snapshot() else {
            return;
        };
        self.load_directory(snapshot.path);
        cx.notify();
    }

    pub(super) fn format_size(size: u64) -> String {
        const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
        let mut value = size as f64;
        let mut unit = 0;
        while value >= 1024. && unit < UNITS.len() - 1 {
            value /= 1024.;
            unit += 1;
        }
        if unit == 0 {
            format!("{size} {}", UNITS[unit])
        } else {
            format!("{value:.1} {}", UNITS[unit])
        }
    }

    pub(super) fn format_modified(timestamp: Option<u32>) -> String {
        timestamp
            .and_then(|timestamp| DateTime::from_timestamp(timestamp as i64, 0))
            .map(|time| {
                time.with_timezone(&Local)
                    .format("%Y-%m-%d %H:%M")
                    .to_string()
            })
            .unwrap_or_else(|| "—".to_owned())
    }

    pub(super) fn format_local_modified(timestamp: Option<SystemTime>) -> String {
        timestamp
            .map(DateTime::<Local>::from)
            .map(|time| time.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "—".to_owned())
    }
    pub fn upload_files() {}
    pub fn download_files() {}
    pub fn open_file_directory() {}
    pub fn open_local_file() {}
    pub fn open_remote_file() {}
}

fn parent_path(path: &str) -> String {
    let path = path.trim_end_matches('/');
    if path.is_empty() {
        return "/".to_owned();
    }
    path.rsplit_once('/')
        .map(|(parent, _)| {
            if parent.is_empty() {
                "/".to_owned()
            } else {
                parent.to_owned()
            }
        })
        .unwrap_or_else(|| ".".to_owned())
}

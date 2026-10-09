use std::{
    path::Path,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

use crate::domain::{sftp::SftpEntry, terminal::TerminalStatus};

use super::super::state::SftpWorkspaceData;
use super::super::{
    DataContext, LocalSnapshot, SftpSnapshot, SftpWatchSummary, TransferRecord, TransferRequest,
    is_transfer_cancellable,
};

#[derive(Clone)]
pub(crate) struct SftpModel {
    workspace_id: String,
    data_context: &'static DataContext,
}

impl SftpModel {
    pub(super) fn new(workspace_id: String, data_context: &'static DataContext) -> Self {
        Self {
            workspace_id,
            data_context,
        }
    }

    pub(crate) fn snapshot(&self) -> SftpSnapshot {
        self.read_state(|state| state.remote.clone())
            .unwrap_or_default()
    }

    pub(crate) fn local_snapshot(&self) -> LocalSnapshot {
        self.read_state(|state| state.local.clone())
            .unwrap_or_default()
    }

    pub(crate) fn set_local_snapshot(&self, snapshot: LocalSnapshot) -> bool {
        self.edit_state(move |state| {
            state.local = snapshot;
            ((), true)
        })
        .is_some()
    }

    pub(crate) fn begin_local_scan(&self, path: &Path, only_if_current: bool) -> Option<u64> {
        self.edit_state(|state| {
            if only_if_current
                && (state.local.path.as_path() != path
                    || state
                        .local_requested_path
                        .as_deref()
                        .is_some_and(|requested| requested != path))
            {
                return (None, false);
            }
            state.local_scan_generation = state.local_scan_generation.wrapping_add(1);
            state.local_requested_path = Some(path.to_owned());
            state.local.loading = true;
            state.local.error = None;
            (Some(state.local_scan_generation), true)
        })?
    }

    pub(crate) fn finish_local_scan(&self, generation: u64, snapshot: LocalSnapshot) -> bool {
        self.edit_state(move |state| {
            if state.local_scan_generation != generation {
                return (false, false);
            }
            state.local = snapshot;
            state.local_requested_path = None;
            (true, true)
        })
        .unwrap_or(false)
    }

    pub(crate) fn fail_local_scan(&self, generation: u64, error: String) {
        let _ = self.edit_state(move |state| {
            if state.local_scan_generation != generation {
                return ((), false);
            }
            state.local.loading = false;
            state.local.error = Some(error);
            state.local_requested_path = None;
            ((), true)
        });
    }

    pub(crate) fn add_watch(&self, watch: SftpWatchSummary) -> bool {
        self.edit_state(move |state| {
            state
                .watches
                .retain(|existing| existing.local_path != watch.local_path);
            state.watches.push(watch);
            ((), true)
        })
        .is_some()
    }

    pub(crate) fn remove_watch(&self, local_path: &str) -> bool {
        self.edit_state(|state| {
            let previous_count = state.watches.len();
            state.watches.retain(|watch| watch.local_path != local_path);
            ((), state.watches.len() != previous_count)
        })
        .is_some()
    }

    pub(crate) fn update(&self, update: impl FnOnce(&mut SftpSnapshot)) {
        let _ = self.edit_state(move |state| {
            update(&mut state.remote);
            state.remote_revision = state.remote_revision.wrapping_add(1);
            ((), true)
        });
    }

    pub(crate) fn set_connected(&self, path: String, entries: Vec<SftpEntry>) {
        self.update(move |snapshot| {
            snapshot.status = TerminalStatus::Connected;
            snapshot.path = path;
            snapshot.entries = std::sync::Arc::new(entries);
            snapshot.loading = false;
            snapshot.error = None;
        });
    }

    pub(crate) fn set_loading(&self) {
        let _ = self.edit_state(|state| {
            state.remote_navigation_generation = state.remote_navigation_generation.wrapping_add(1);
            state.remote.loading = true;
            state.remote.error = None;
            state.remote_revision = state.remote_revision.wrapping_add(1);
            ((), true)
        });
    }

    pub(crate) fn remote_refresh_generation(&self, path: &str) -> Option<u64> {
        self.read_state(|state| {
            (state.remote.path == path).then_some(state.remote_navigation_generation)
        })?
    }

    pub(crate) fn set_directory_if_current(
        &self,
        path: String,
        generation: u64,
        entries: Vec<SftpEntry>,
    ) -> bool {
        self.edit_state(move |state| {
            if state.remote.path != path || state.remote_navigation_generation != generation {
                return (false, false);
            }
            state.remote.path = path;
            state.remote.entries = std::sync::Arc::new(entries);
            state.remote.loading = false;
            state.remote.error = None;
            state.remote_revision = state.remote_revision.wrapping_add(1);
            (true, true)
        })
        .unwrap_or(false)
    }

    pub(crate) fn set_error_if_current(&self, path: &str, generation: u64, error: String) {
        let _ = self.edit_state(move |state| {
            if state.remote.path != path || state.remote_navigation_generation != generation {
                return ((), false);
            }
            state.remote.loading = false;
            state.remote.error = Some(error);
            state.remote_revision = state.remote_revision.wrapping_add(1);
            ((), true)
        });
    }

    pub(crate) fn set_directory(&self, path: String, entries: Vec<SftpEntry>) {
        self.update(move |snapshot| {
            snapshot.path = path;
            snapshot.entries = std::sync::Arc::new(entries);
            snapshot.loading = false;
            snapshot.error = None;
        });
    }

    pub(crate) fn set_error(&self, error: String) {
        self.update(|snapshot| {
            snapshot.loading = false;
            snapshot.error = Some(error);
        });
    }

    pub(crate) fn set_failed(&self, error: String) {
        self.update(|snapshot| {
            snapshot.status = TerminalStatus::Failed;
            snapshot.loading = false;
            snapshot.error = Some(error);
        });
    }

    pub(crate) fn next_transfer_id(&self) -> u64 {
        self.data_context
            .next_sftp_transfer_id
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.wrapping_add(1).max(1))
            })
            .unwrap_or(1)
    }

    pub(crate) fn add_transfer(&self, transfer: TransferRecord) -> bool {
        self.edit_state(move |state| {
            state.transfers.push(transfer);
            ((), true)
        })
        .is_some()
    }

    pub(crate) fn transfer_records(&self, ids: &[u64]) -> Vec<TransferRecord> {
        self.read_state(|state| {
            state
                .transfers
                .iter()
                .filter(|transfer| ids.contains(&transfer.id))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
    }

    pub(crate) fn find_retryable_transfer(&self, transfer_id: u64) -> Option<TransferRecord> {
        self.read_state(|state| {
            state
                .transfers
                .iter()
                .find(|transfer| {
                    transfer.id == transfer_id
                        && matches!(transfer.status.as_str(), "失败" | "已取消")
                })
                .cloned()
        })?
    }

    pub(crate) fn update_transfer(
        &self,
        transfer_id: u64,
        progress: f32,
        transferred: u64,
        total: u64,
        status: impl Into<String>,
    ) {
        let status = status.into();
        let is_progress = status == "传输中" && transferred > 0;
        let now = Instant::now();
        let _ = self.edit_state(move |state| {
            if let Some(transfer) = state
                .transfers
                .iter_mut()
                .find(|transfer| transfer.id == transfer_id)
            {
                transfer.progress = progress.clamp(0., 1.);
                if status == "扫描中" {
                    transfer.speed = 0;
                    transfer.transferred_bytes = 0;
                    transfer.total_bytes = 0;
                    transfer.error = None;
                    transfer.started_at = None;
                    transfer.speed_updated_at = None;
                } else if transferred > 0 {
                    transfer.transferred_bytes = transferred;
                    transfer.total_bytes = total;
                    let started_at = transfer.started_at.get_or_insert(now);
                    let elapsed = now.duration_since(*started_at).as_secs_f64();
                    let should_update = transfer.speed_updated_at.is_none_or(|updated_at| {
                        now.duration_since(updated_at) >= Duration::from_secs(1)
                    });
                    if elapsed > 0. && should_update {
                        transfer.speed = (transferred as f64 / elapsed) as u64;
                        transfer.speed_updated_at = Some(now);
                    }
                }
                transfer.status = status;
            }

            let notify = if is_progress {
                let notify = state
                    .last_transfer_notification
                    .is_none_or(|updated| now.duration_since(updated) >= Duration::from_secs(1));
                if notify {
                    state.last_transfer_notification = Some(now);
                }
                notify
            } else {
                true
            };
            ((), notify)
        });
    }

    pub(crate) fn request_cancel(&self, transfer_id: u64) -> bool {
        self.edit_state(move |state| {
            if !state.transfers.iter().any(|transfer| {
                transfer.id == transfer_id && is_transfer_cancellable(&transfer.status)
            }) {
                return (false, false);
            }
            state.cancelled_transfers.insert(transfer_id);
            set_transfer_status(state, transfer_id, "已取消");
            (true, true)
        })
        .unwrap_or(false)
    }

    pub(crate) fn is_cancelled(&self, transfer_id: u64) -> bool {
        self.read_state(|state| state.cancelled_transfers.contains(&transfer_id))
            .unwrap_or(false)
    }

    pub(crate) fn set_transfer_error(&self, transfer_id: u64, error: String) {
        let _ = self.edit_state(move |state| {
            if let Some(transfer) = state
                .transfers
                .iter_mut()
                .find(|transfer| transfer.id == transfer_id)
            {
                transfer.status = "失败".to_owned();
                transfer.error = Some(error);
                transfer.speed = 0;
            }
            ((), true)
        });
    }

    pub(crate) fn set_upload_directory(&self, transfer_id: u64, is_directory: bool) {
        let _ = self.edit_state(move |state| {
            if let Some(transfer) = state
                .transfers
                .iter_mut()
                .find(|transfer| transfer.id == transfer_id)
            {
                if let TransferRequest::Upload {
                    is_directory: current,
                    ..
                } = &mut transfer.request
                {
                    *current = is_directory;
                }
            }
            ((), true)
        });
    }

    fn read_state<R>(&self, read: impl FnOnce(&SftpWorkspaceData) -> R) -> Option<R> {
        self.data_context
            .sftp
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(&self.workspace_id)
            .map(read)
    }

    fn edit_state<R>(&self, edit: impl FnOnce(&mut SftpWorkspaceData) -> (R, bool)) -> Option<R> {
        self.data_context.edit_sftp_state(&self.workspace_id, edit)
    }
}

fn set_transfer_status(state: &mut SftpWorkspaceData, transfer_id: u64, status: &str) {
    if let Some(transfer) = state
        .transfers
        .iter_mut()
        .find(|transfer| transfer.id == transfer_id)
    {
        transfer.status = status.to_owned();
        if status == "已取消" {
            transfer.speed = 0;
        }
    }
}

use std::sync::{
    RwLock, RwLockReadGuard,
    atomic::{AtomicU64, Ordering},
};

use crate::domain::terminal::TerminalData;

pub(in crate::gui::workspace) struct TerminalModel {
    data: RwLock<TerminalData>,
    mcp_snapshot_version: AtomicU64,
    gui_snapshot_version: AtomicU64,
}

impl TerminalModel {
    pub(crate) fn new(data: TerminalData) -> Self {
        Self {
            data: RwLock::new(data),
            mcp_snapshot_version: AtomicU64::new(0),
            gui_snapshot_version: AtomicU64::new(0),
        }
    }

    pub(crate) fn read(&self) -> RwLockReadGuard<'_, TerminalData> {
        self.data
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn replace(
        &self,
        data: TerminalData,
        mcp_snapshot_version: u64,
        gui_snapshot_version: u64,
    ) {
        *self
            .data
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = data;
        self.mcp_snapshot_version
            .store(mcp_snapshot_version, Ordering::Release);
        self.gui_snapshot_version
            .store(gui_snapshot_version, Ordering::Release);
    }

    pub(crate) fn mcp_snapshot_version(&self) -> u64 {
        self.mcp_snapshot_version.load(Ordering::Acquire)
    }

    pub(crate) fn gui_snapshot_version(&self) -> u64 {
        self.gui_snapshot_version.load(Ordering::Acquire)
    }
}

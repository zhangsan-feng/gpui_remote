use std::sync::{
    RwLock, RwLockReadGuard,
    atomic::{AtomicU64, Ordering},
};

use crate::domain::terminal::TerminalData;

pub(in crate::gui::workspace) struct TerminalModel {
    data: RwLock<TerminalData>,
    revision: AtomicU64,
}

impl TerminalModel {
    pub(crate) fn new(data: TerminalData) -> Self {
        Self {
            data: RwLock::new(data),
            revision: AtomicU64::new(0),
        }
    }

    pub(crate) fn read(&self) -> RwLockReadGuard<'_, TerminalData> {
        self.data
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(crate) fn replace(&self, data: TerminalData, revision: u64) {
        *self
            .data
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = data;
        self.revision.store(revision, Ordering::Release);
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }
}

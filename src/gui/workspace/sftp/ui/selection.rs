use std::{collections::HashSet, hash::Hash, path::PathBuf};

use gpui::Context;

use super::super::SftpView;

#[derive(Clone, Debug, Default)]
pub struct MultiSelection<T> {
    selected: HashSet<T>,
}

impl<T: Eq + Hash> MultiSelection<T> {
    pub(crate) fn contains(&self, value: &T) -> bool {
        self.selected.contains(value)
    }

    pub(crate) fn values(&self) -> Vec<T>
    where
        T: Clone,
    {
        self.selected.iter().cloned().collect()
    }

    pub(crate) fn toggle(&mut self, value: T, additive: bool) {
        if !additive {
            self.selected.clear();
        }
        if !self.selected.remove(&value) {
            self.selected.insert(value);
        }
    }

    pub(crate) fn clear(&mut self) {
        self.selected.clear();
    }
}

impl SftpView {
    pub(super) fn mark_drag_started(&mut self) {
        self.drag_started = true;
    }

    pub(super) fn finish_drag(&mut self) {
        self.drag_started = false;
    }

    pub(super) fn finish_local_click(
        &mut self,
        path: PathBuf,
        selected: bool,
        additive: bool,
        cx: &mut Context<Self>,
    ) {
        let was_dragging = self.drag_started;
        self.drag_started = false;
        if selected && !additive && !was_dragging {
            self.select_local_path(path, false, cx);
        }
    }

    pub(super) fn finish_remote_click(
        &mut self,
        path: String,
        selected: bool,
        additive: bool,
        cx: &mut Context<Self>,
    ) {
        let was_dragging = self.drag_started;
        self.drag_started = false;
        if selected && !additive && !was_dragging {
            self.select_remote_path(path, false, cx);
        }
    }

    pub(super) fn select_local_path(
        &mut self,
        path: PathBuf,
        additive: bool,
        cx: &mut Context<Self>,
    ) {
        self.local_selection.toggle(path, additive);
        cx.notify();
    }

    pub(super) fn select_remote_path(
        &mut self,
        path: String,
        additive: bool,
        cx: &mut Context<Self>,
    ) {
        self.remote_selection.toggle(path, additive);
        cx.notify();
    }
}

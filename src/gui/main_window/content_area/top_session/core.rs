use gpui_kit::{Context, Pixels, Point, Window};

use super::WorkspaceSession;

impl WorkspaceSession {
    pub fn activate(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.selected_id.as_deref() == Some(id)
            || !self.sessions.iter().any(|item| item.id == id)
        {
            return;
        }
        self.emit_selected_workspace(Some(id.to_owned()), cx);
    }

    pub fn close(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.sessions.iter().any(|item| item.id == id) {
            return;
        }
        self.emit_closed_workspace(id.to_owned(), cx);
    }
}

impl WorkspaceSession {
    pub(super) fn move_tab(&mut self, from: &str, target: &str, cx: &mut Context<Self>) {
        let Some(from_index) = self.sessions.iter().position(|session| session.id == from) else {
            return;
        };
        let Some(target_index) = self
            .sessions
            .iter()
            .position(|session| session.id == target)
        else {
            return;
        };
        if from_index == target_index {
            self.set_drop_target(None, cx);
            return;
        }
        self.drop_target = None;
        let to = target_index;
        let session = self.sessions.remove(from_index);
        self.sessions.insert(to, session);
        log::debug!(
            "Workspace tab drag completed: workspace_id={from}, from={from_index}, to={to}, selected={}",
            self.selected_id.as_deref() == Some(from)
        );
        cx.notify();
    }
}

impl WorkspaceSession {
    pub(super) fn set_drop_target_for(&mut self, from: &str, target: &str, cx: &mut Context<Self>) {
        let from_index = self.sessions.iter().position(|session| session.id == from);
        let target_index = self
            .sessions
            .iter()
            .position(|session| session.id == target);
        // Use the whole tab so either half accepts the same reorder operation.
        let target = from_index
            .zip(target_index)
            .and_then(|(from, to)| (from != to).then(|| (target.to_owned(), from < to)));
        self.set_drop_target(target, cx);
    }

    pub(super) fn set_drop_target(
        &mut self,
        target: Option<(String, bool)>,
        cx: &mut Context<Self>,
    ) {
        if self.drop_target != target {
            log::debug!("Workspace tab drag target changed: target={target:?}");
            self.drop_target = target;
            cx.notify();
        }
    }
}

impl WorkspaceSession {
    pub(super) fn finish_drag_on(
        &mut self,
        target: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !cx.has_active_drag() {
            return;
        }
        let Some(from) = self.drag_source.take() else {
            return;
        };
        log::debug!(
            "Reorder release captured: source={from}, target={target}, position={:?}",
            window.mouse_position()
        );
        self.move_tab(&from, target, cx);
        cx.stop_active_drag(window);
        cx.stop_propagation();
    }

    pub(super) fn clear_drag(&mut self, cx: &mut Context<Self>) {
        log::debug!(
            "tab_drag_clear_scheduled source={:?} target={:?} active={}",
            self.drag_source,
            self.drop_target,
            cx.has_active_drag()
        );
        self.set_drop_target(None, cx);
        // Outside-release handlers run in capture too; retain the source until
        // the target's capture handler has had a chance to finish this event.
        let owner = cx.entity();
        cx.defer(move |cx| {
            owner.update(cx, |this, _| {
                log::debug!(
                    "tab_drag_clear_deferred source={:?} target={:?}",
                    this.drag_source,
                    this.drop_target
                );
                this.drag_source = None;
            })
        });
    }
}

impl WorkspaceSession {
    pub(super) fn trailing_drop_target(&self, position: Point<Pixels>) -> Option<String> {
        if !self.tabs_scroll.bounds().contains(&position) {
            return None;
        }
        let last_index = self.tabs_scroll.children_count().checked_sub(1)?;
        let last_bounds = self.tabs_scroll.bounds_for_item(last_index)?;
        if position.x < last_bounds.right() + self.tabs_scroll.offset().x {
            return None;
        }
        self.sessions.last().map(|session| session.id.clone())
    }

    pub(super) fn finish_drag_in_blank(
        &mut self,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.drag_source.is_none() || !cx.has_active_drag() {
            return;
        }
        if let Some(target) = self.trailing_drop_target(position) {
            log::debug!(
                "tab_drag_release_blank source={:?} target={target} position={position:?}",
                self.drag_source
            );
            self.finish_drag_on(&target, window, cx);
        }
    }
}

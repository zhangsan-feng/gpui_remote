use gpui_kit::{Context, Window};

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
    pub(super) fn move_tab(
        &mut self,
        from: &str,
        target: &str,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
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
        let after = self
            .drop_target
            .as_ref()
            .filter(|(id, _)| id == target)
            .map(|(_, after)| *after)
            .unwrap_or_else(|| {
                self.tabs_scroll
                    .bounds_for_item(target_index)
                    .is_some_and(|bounds| {
                        window.mouse_position().x > bounds.center().x + self.tabs_scroll.offset().x
                    })
            });
        self.drop_target = None;
        let insertion = target_index + usize::from(after);
        let to = insertion - usize::from(from_index < insertion);
        let session = self.sessions.remove(from_index);
        self.sessions.insert(to, session);
        log::info!(
            "Workspace tab drag completed: workspace_id={from}, from={from_index}, to={to}, selected={}",
            self.selected_id.as_deref() == Some(from)
        );
        cx.notify();
    }
}

impl WorkspaceSession {
    pub(super) fn set_drop_target(
        &mut self,
        target: Option<(String, bool)>,
        cx: &mut Context<Self>,
    ) {
        if self.drop_target != target {
            self.drop_target = target;
            cx.notify();
        }
    }
}

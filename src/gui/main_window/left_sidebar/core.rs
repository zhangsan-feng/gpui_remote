use anyhow::{Context as _, Result, anyhow};
use gpui_kit::{Context, Pixels, Point, Window};

use crate::application::session::model::SessionSummary;

use super::{SessionComponent, tree::SidebarRow};

impl SessionComponent {
    pub(super) fn refresh_sessions(&mut self, cx: &mut Context<Self>) {
        self.refresh_generation = self.refresh_generation.wrapping_add(1);
        let refresh_generation = self.refresh_generation;
        let application = crate::application::APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            let result = crate::application::run_application(async move {
                application.sessions.session_sidebar().await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                if this.refresh_generation != refresh_generation {
                    return;
                }
                match result {
                    Ok(mut sidebar) => {
                        sidebar.sessions.sort_by_key(|session| {
                            this.sessions
                                .iter()
                                .position(|old| old.id == session.id)
                                .unwrap_or(usize::MAX)
                        });
                        this.sessions = sidebar.sessions;
                        this.groups = sidebar.groups;
                        this.core_err = None;
                        cx.notify();
                    }
                    Err(error) => this.set_error(anyhow!(error), cx),
                }
            });
        })
        .detach();
    }

    pub(super) fn reveal_group(&mut self, group_id: Option<&str>) {
        let mut current = group_id;
        while let Some(id) = current {
            self.collapsed_groups.remove(id);
            current = self
                .groups
                .iter()
                .find(|group| group.id == id)
                .and_then(|group| group.parent_id.as_deref());
        }
    }

    pub(super) fn find_session_in_projection(&self, id: &str) -> Result<SessionSummary> {
        self.sessions
            .iter()
            .into_iter()
            .find(|session| session.id == id)
            .with_context(|| format!("top_session not found: {id}"))
            .cloned()
    }
}

impl SessionComponent {
    pub(super) fn move_session(&mut self, from: &str, target: &str, cx: &mut Context<Self>) {
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
        self.drop_target = None;
        if from_index != target_index
            && self.sessions[from_index].group_id == self.sessions[target_index].group_id
        {
            let session = self.sessions.remove(from_index);
            self.sessions.insert(target_index, session);
            log::debug!(
                "Session drag completed: session_id={from}, from={from_index}, to={target_index}"
            );
        }
        cx.notify();
    }
}

impl SessionComponent {
    pub(super) fn set_drop_target_for(&mut self, from: &str, target: &str, cx: &mut Context<Self>) {
        let from_index = self.sessions.iter().position(|session| session.id == from);
        let target_index = self
            .sessions
            .iter()
            .position(|session| session.id == target);
        // The whole row is a target: moving down inserts after it, moving up before it.
        let target = from_index.zip(target_index).and_then(|(from, to)| {
            (from != to && self.sessions[from].group_id == self.sessions[to].group_id)
                .then(|| (target.to_owned(), from < to))
        });
        self.set_drop_target(target, cx);
    }

    pub(super) fn set_drop_target(
        &mut self,
        target: Option<(String, bool)>,
        cx: &mut Context<Self>,
    ) {
        if self.drop_target != target {
            log::debug!("Session drag target changed: target={target:?}");
            self.drop_target = target;
            cx.notify();
        }
    }
}

impl SessionComponent {
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
        self.move_session(&from, target, cx);
        cx.stop_active_drag(window);
        cx.stop_propagation();
    }

    pub(super) fn clear_drag(&mut self, cx: &mut Context<Self>) {
        log::debug!(
            "session_drag_clear_scheduled source={:?} target={:?} active={}",
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
                    "session_drag_clear_deferred source={:?} target={:?}",
                    this.drag_source,
                    this.drop_target
                );
                this.drag_source = None;
            })
        });
    }
}

impl SessionComponent {
    pub(super) fn trailing_drop_target(
        &self,
        position: Point<Pixels>,
        cx: &Context<Self>,
    ) -> Option<String> {
        if !self.scroll.bounds().contains(&position) {
            return None;
        }
        let last_index = self.scroll.children_count().checked_sub(1)?;
        let last_bounds = self.scroll.bounds_for_item(last_index)?;
        if position.y < last_bounds.bottom() + self.scroll.offset().y {
            return None;
        }
        match self.visible_rows(cx).last()? {
            SidebarRow::Session(session, _) => Some(session.id.clone()),
            SidebarRow::Group(..) => None,
        }
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
        if let Some(target) = self.trailing_drop_target(position, cx) {
            log::debug!(
                "session_drag_release_blank source={:?} target={target} position={position:?}",
                self.drag_source
            );
            self.finish_drag_on(&target, window, cx);
        }
    }
}

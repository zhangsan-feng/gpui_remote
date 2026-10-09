use gpui_kit::*;

use super::ServicesOperationWindow;
use crate::domain::port_forward::PortForwardRule;

pub(super) enum PortForwardAction {
    Enable(String, bool),
    Retry(String),
    Delete(String),
}

impl ServicesOperationWindow {
    pub(super) fn refresh_port_forward_rules(&mut self, cx: &mut Context<Self>) {
        self.port_forward_busy = true;
        let application = crate::application::APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                application.port_forward.list().await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                this.port_forward_busy = false;
                match result {
                    Ok(rules) => {
                        this.port_forward_rules = rules;
                        this.port_forward_error = None;
                    }
                    Err(error) => this.port_forward_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn edit_port_forward_rule(
        &mut self,
        rule: Option<PortForwardRule>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.port_forward_busy || rule.as_ref().is_some_and(|rule| rule.enabled) {
            return;
        }
        super::port_forward_rule_window::open_rule_window(rule, cx.weak_entity(), window, cx);
    }

    pub(super) fn apply_port_forward_action(
        &mut self,
        action: PortForwardAction,
        cx: &mut Context<Self>,
    ) {
        if self.port_forward_busy {
            return;
        }
        self.port_forward_busy = true;
        self.port_forward_error = None;
        self.port_forward_delete_confirmation = None;
        let application = crate::application::APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            let (result, rules) = crate::global_state::run_application(async move {
                let result = match action {
                    PortForwardAction::Enable(id, enabled) => {
                        application.port_forward.set_enabled(id, enabled).await
                    }
                    PortForwardAction::Retry(id) => application.port_forward.retry(id).await,
                    PortForwardAction::Delete(id) => application.port_forward.delete(id).await,
                };
                // Enablement is persisted even if starting fails. Always reload it.
                Ok((result, application.port_forward.list().await))
            })
            .await
            .unwrap_or_else(|error| (Err(error.clone()), Err(error)));
            let _ = this.update(cx, |this, cx| {
                this.port_forward_busy = false;
                if let Ok(rules) = rules.as_ref() {
                    this.port_forward_rules = rules.clone();
                }
                this.port_forward_error = result.err().or_else(|| rules.err());
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

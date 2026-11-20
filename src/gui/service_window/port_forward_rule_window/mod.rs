use super::ServicesOperationWindow;
use crate::{
    application::port_forward::PortForwardRuleDraft, domain::port_forward::PortForwardRule,
};
use gpui_kit::component::input::InputState;
use gpui_kit::*;

mod core;
mod external;
mod ui;
pub(super) use external::open_rule_window;

struct PortForwardRuleWindow {
    id: Option<String>,
    listen_host: String,
    listen_port: Entity<InputState>,
    target_host: Entity<InputState>,
    target_port: Entity<InputState>,
    parent: WeakEntity<ServicesOperationWindow>,
    busy: bool,
    error: Option<String>,
}

impl PortForwardRuleWindow {
    fn new(
        rule: Option<PortForwardRule>,
        parent: WeakEntity<ServicesOperationWindow>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let draft = rule
            .as_ref()
            .map(|rule| PortForwardRuleDraft {
                listen_host: rule.listen_host.clone(),
                listen_port: rule.listen_port.to_string(),
                target_host: rule.target_host.clone(),
                target_port: rule.target_port.to_string(),
            })
            .unwrap_or_default();
        let mut input = |value: String, placeholder: &'static str| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(value)
                    .placeholder(placeholder)
            })
        };
        Self {
            id: rule.map(|rule| rule.id),
            listen_host: draft.listen_host,
            listen_port: input(draft.listen_port, "1-65535"),
            target_host: input(draft.target_host, "目标 IP 或域名"),
            target_port: input(draft.target_port, "1-65535"),
            parent,
            busy: false,
            error: None,
        }
    }
}

use crate::application::port_forward::PortForwardRuleDraft;
use gpui_kit::*;

use super::PortForwardRuleWindow;

impl PortForwardRuleWindow {
    pub(super) fn save(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.error = None;
        let draft = PortForwardRuleDraft {
            listen_host: self.listen_host.clone(),
            listen_port: self.listen_port.read(cx).value().to_string(),
            target_host: self.target_host.read(cx).value().to_string(),
            target_port: self.target_port.read(cx).value().to_string(),
        };
        let id = self.id.clone();
        let application = crate::application::APPLICATION.clone();
        let handle = window.window_handle();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                match id {
                    Some(id) => application.port_forward.update(id, draft).await,
                    None => application.port_forward.create(draft).await,
                }
            })
            .await;
            match result {
                Ok(_) => {
                    let _ = this.update(cx, |this, cx| {
                        let _ = this
                            .parent
                            .update(cx, |parent, cx| parent.refresh_port_forward_rules(cx));
                    });
                    let _ = cx.update_window(handle, |_, window, _| window.remove_window());
                }
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.busy = false;
                        this.error = Some(error);
                        cx.notify();
                    });
                }
            }
        })
        .detach();
        cx.notify();
    }
}

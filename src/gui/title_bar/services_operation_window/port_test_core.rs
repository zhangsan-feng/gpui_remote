use gpui_kit::*;

use super::ServicesOperationWindow;

impl ServicesOperationWindow {
    pub(super) fn run_port_test(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.port_test_busy {
            return;
        }

        let host = self.port_test_host.read(cx).value().to_string();
        let port = self.port_test_port.read(cx).value().to_string();
        self.port_test_busy = true;
        self.port_test_error = None;
        self.port_test_result = None;
        cx.notify();

        let application = crate::application::APPLICATION.clone();
        cx.spawn(async move |this, cx| {
            let result = crate::global_state::run_application(async move {
                application.port_test.test_tcp_port(host, port).await
            })
            .await;
            let _ = this.update(cx, |this, cx| {
                this.port_test_busy = false;
                match result {
                    Ok(result) => {
                        this.port_test_error = None;
                        this.port_test_result = Some(result);
                    }
                    Err(error) => this.port_test_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

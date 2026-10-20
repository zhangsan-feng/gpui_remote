mod core;
mod ui;

use gpui_kit::component::input::InputState;
use gpui_kit::*;

pub(super) struct NetworkConfigEditor {
    proxy: bool,
    editing: Option<String>,
    show_editor: bool,
    name: Entity<InputState>,
    address: Entity<InputState>,
    port: Entity<InputState>,
    username: Entity<InputState>,
    password: Entity<InputState>,
    forward_port: Entity<InputState>,
    busy: bool,
    message: Option<String>,
    failed: bool,
}

impl NetworkConfigEditor {
    pub(super) fn new(proxy: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut updates = crate::data_context::DATA_CONTEXT
            .notice
            .subscribe_gui_refresh();
        cx.spawn(async move |this, cx| {
            while updates.changed().await.is_ok() {
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            proxy,
            editing: None,
            show_editor: false,
            name: Self::input("配置名称", "", false, window, cx),
            address: Self::input(
                if proxy {
                    "代理主机"
                } else {
                    "转发地址"
                },
                if proxy { "" } else { "127.0.0.1" },
                false,
                window,
                cx,
            ),
            port: Self::input(
                if proxy {
                    "代理端口"
                } else {
                    "远端监听端口"
                },
                if proxy { "1080" } else { "" },
                false,
                window,
                cx,
            ),
            username: Self::input("代理用户名（可选）", "", false, window, cx),
            password: Self::input("代理密码（可选）", "", true, window, cx),
            forward_port: Self::input("转发端口", "", false, window, cx),
            busy: false,
            message: None,
            failed: false,
        }
    }

    fn input(
        placeholder: &'static str,
        value: &str,
        masked: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(value.to_owned())
                .masked(masked)
        })
    }
}

impl Render for NetworkConfigEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_view(cx)
    }
}

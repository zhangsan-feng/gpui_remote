use super::{PortForwardRuleWindow, ServicesOperationWindow};
use crate::domain::port_forward::PortForwardRule;
use gpui_kit::*;

pub(crate) fn open_rule_window(
    rule: Option<PortForwardRule>,
    parent: WeakEntity<ServicesOperationWindow>,
    window: &mut Window,
    cx: &mut App,
) {
    let mut options =
        crate::component::window::window_center_options(window, size(px(460.), px(520.)));
    options.titlebar = Some(TitlebarOptions {
        title: Some("端口转发规则".into()),
        appears_transparent: false,
        traffic_light_position: None,
    });
    options.kind = WindowKind::Dialog;
    options.is_resizable = false;
    options.is_minimizable = false;
    if let Err(error) = cx.open_window(options, move |window, cx| {
        window.on_window_should_close(cx, |window, _| {
            window.remove_window();
            false
        });
        let editor = cx.new(|cx| PortForwardRuleWindow::new(rule, parent, window, cx));
        cx.new(|cx| gpui_kit::component::Root::new(editor, window, cx))
    }) {
        log::error!("打开端口转发规则窗口失败: {error}");
    }
}

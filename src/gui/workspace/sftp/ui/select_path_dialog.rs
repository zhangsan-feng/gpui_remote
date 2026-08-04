use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    v_flex, ActiveTheme, Sizable,
};

use super::super::SftpView;

#[derive(Clone, Copy)]
pub enum PathTarget {
    Local,
    Remote,
}

pub(super) struct PathInputDialog {
    parent: WeakEntity<SftpView>,
    target: PathTarget,
    input: Entity<InputState>,
}

impl PathInputDialog {
    fn new(
        parent: WeakEntity<SftpView>,
        target: PathTarget,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).default_value(path));
        input.update(cx, |input, cx| {
            let length = input.text().len();
            input.set_selected_range(0..length, cx);
            input.focus(window, cx);
        });
        Self {
            parent,
            target,
            input,
        }
    }

    fn confirm(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let path = self.input.read(cx).value().trim().to_owned();
        if path.is_empty() {
            window.remove_window();
            return;
        }

        let target = self.target;
        let _ = self.parent.update(cx, |view, cx| match target {
            PathTarget::Local => view.open_local_directory(path.into(), cx),
            PathTarget::Remote => view.open_directory(path, cx),
        });
        window.remove_window();
    }

    fn cancel(&mut self, _: &ClickEvent, window: &mut Window, _: &mut Context<Self>) {
        window.remove_window();
    }
}

impl SftpView {
    pub(super) fn open_path_dialog(
        &self,
        target: PathTarget,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let window_size = size(px(560.), px(170.));
        let title = match target {
            PathTarget::Local => "打开本地路径",
            PathTarget::Remote => "打开远程路径",
        };
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(window_size, cx)),
            window_min_size: Some(window_size),
            titlebar: Some(TitlebarOptions {
                title: Some(title.into()),
                appears_transparent: false,
                traffic_light_position: None,
            }),
            kind: WindowKind::Dialog,
            is_resizable: false,
            is_minimizable: false,
            ..Default::default()
        };
        let parent = cx.weak_entity();
        let _ = cx.open_window(options, move |window, cx| {
            let dialog = cx.new(|cx| PathInputDialog::new(parent, target, path, window, cx));
            cx.new(|cx| { gpui_component::Root::new(dialog, window, cx) })
        });
    }
}

impl Render for PathInputDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        v_flex()
            .size_full()
            .gap_4()
            .p_5()
            .bg(colors.background)
            .text_color(colors.foreground)
            .child(Input::new(&self.input).small())
            .child(
                h_flex()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("sftp-path-cancel")
                            .ghost()
                            .small()
                            .label("取消")
                            .on_click(cx.listener(Self::cancel)),
                    )
                    .child(
                        Button::new("sftp-path-confirm")
                            .primary()
                            .small()
                            .label("确定")
                            .on_click(cx.listener(Self::confirm)),
                    ),
            )
    }
}

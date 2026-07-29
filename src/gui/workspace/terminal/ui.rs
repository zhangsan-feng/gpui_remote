use gpui::*;
use gpui_component::v_flex;

use crate::{
    component::{color::rgb_to_u32, theme},
    domain::terminal::{TerminalSessionCommand, TerminalStatus},
};

use super::*;

impl TerminalView {
    pub(super) fn render_view(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(workspace_id) = self.selected_workspace_id.clone() else {
            return div().size_full().into_any_element();
        };
        let Some(terminal_model) = self.model(&workspace_id) else {
            return div().size_full().into_any_element();
        };
        self.observed_revision = Some((workspace_id.clone(), terminal_model.revision()));
        let (frame, status, message) = {
            let terminal = terminal_model.read();
            (
                terminal.frame.clone(),
                terminal.status.clone(),
                terminal.message.clone(),
            )
        };
        self.scroll_handle
            .sync(&frame, self.command_sender(&workspace_id));
        self.sync_list(&workspace_id, frame.lines.len().max(1));
        let cell_width = terminal_cell_width(window);

        let focus = self.focus.clone();
        let scroll_commands = self.command_sender(&workspace_id);
        let content = if status == TerminalStatus::Failed {
            render_connection_error(message).into_any_element()
        } else {
            self.render_terminal_list(workspace_id, frame, cell_width, cx)
                .into_any_element()
        };
        v_flex()
            .id("ssh-terminal")
            .role(Role::Terminal)
            .aria_label("SSH 终端")
            .size_full()
            .key_context(TERMINAL_KEY_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::send_tab))
            .on_action(cx.listener(Self::copy_terminal))
            .on_action(cx.listener(Self::paste_terminal))
            .on_key_down(cx.listener(Self::key_down))
            .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                focus.focus(window, cx);
            })
            .on_mouse_up(MouseButton::Left, cx.listener(Self::finish_text_selection))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::finish_text_selection))
            .on_scroll_wheel(move |event, _, cx| {
                let pixels = event.delta.pixel_delta(px(TERMINAL_LINE_HEIGHT)).y;
                let lines = (f32::from(pixels) / TERMINAL_LINE_HEIGHT).round() as i32;
                if lines != 0 {
                    if let Some(commands) = &scroll_commands {
                        let _ = commands.send(TerminalSessionCommand::Scroll { lines });
                    }
                    cx.stop_propagation();
                }
            })
            .bg(theme::terminal_color(cx))
            .text_color(contrast_text(theme::terminal_base_color(cx)))
            .child(content)
            .into_any_element()
    }

    fn sync_list(&mut self, workspace_id: &str, line_count: usize) {
        if self.listed_workspace_id.as_deref() != Some(workspace_id) {
            self.list_state
                .reset_with_uniform_height(line_count, px(TERMINAL_LINE_HEIGHT));
            self.list_state.set_follow_mode(FollowMode::Tail);
            self.listed_workspace_id = Some(workspace_id.to_owned());
            return;
        }

        let previous_count = self.list_state.item_count();
        if line_count > previous_count {
            self.list_state
                .splice(previous_count..previous_count, line_count - previous_count);
        } else if line_count < previous_count {
            self.list_state
                .reset_with_uniform_height(line_count, px(TERMINAL_LINE_HEIGHT));
        }
    }

    pub(super) fn sync_pty_size(
        &mut self,
        workspace_id: &str,
        viewport: Size<Pixels>,
        cell_width: Pixels,
    ) {
        let text_width = f32::from(viewport.width)
            - terminal_render::GUTTER_WIDTH
            - scroll::SCROLLBAR_WIDTH
            - TERMINAL_TEXT_HORIZONTAL_PADDING;
        let columns = (text_width / f32::from(cell_width)).floor().max(20.0) as u32;
        let rows = (f32::from(viewport.height) / TERMINAL_LINE_HEIGHT)
            .floor()
            .max(6.0) as u32;
        let pty_size = (workspace_id.to_owned(), columns, rows);
        if self.last_pty_size.as_ref() != Some(&pty_size) {
            self.resize(workspace_id, columns, rows);
            self.last_pty_size = Some(pty_size);
        }
    }
}

fn contrast_text(background: Hsla) -> Hsla {
    if background.l < 0.48 {
        rgb_to_u32(226, 232, 240).into()
    } else {
        rgb_to_u32(30, 41, 59).into()
    }
}

fn render_connection_error(message: Option<String>) -> impl IntoElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .px_8()
        .child(
            v_flex()
                .max_w(px(640.))
                .items_center()
                .gap_3()
                .rounded_lg()
                .border_1()
                .border_color(rgb_to_u32(127, 29, 29))
                .bg(rgb_to_u32(30, 18, 24))
                .px_6()
                .py_5()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb_to_u32(248, 113, 113))
                        .child("连接失败"),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb_to_u32(203, 213, 225))
                        .text_center()
                        .child(message.unwrap_or_else(|| "未提供错误原因".to_owned())),
                ),
        )
}

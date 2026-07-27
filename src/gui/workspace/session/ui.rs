use gpui::*;
use gpui_component::h_flex;

use crate::{
    component::color::rgb_to_u32,
    domain::{
        session::{SessionProfile, WorkspaceSession as WorkspaceSessionConnection},
        terminal::TerminalStatus,
    },
};

use super::WorkspaceSession;

pub(in crate::gui::workspace) fn workspace_tab(
    workspace_session: WorkspaceSessionConnection,
    profile: SessionProfile,
    status: TerminalStatus,
    active: bool,
    workspace: Entity<WorkspaceSession>,
) -> impl IntoElement {
    let session_id = workspace_session.id.clone();
    let close_id = workspace_session.id;
    let (status_color, status_text) = match status {
        TerminalStatus::Connecting => (rgb_to_u32(217, 119, 6), "连接中"),
        TerminalStatus::Connected => (rgb_to_u32(22, 163, 74), "已连接"),
        TerminalStatus::Disconnected => (rgb_to_u32(107, 114, 128), "已断开"),
        TerminalStatus::Failed => (rgb_to_u32(220, 38, 38), "失败"),
    };
    h_flex()
        .id(format!("workspace-tab-{session_id}"))
        .w(px(220.))
        .h(px(34.))
        .ml_1()
        .pl_3()
        .pr_2()
        .gap_2()
        .cursor_pointer()
        .rounded_lg()
        .border_1()
        .border_color(rgb_to_u32(226, 219, 232))
        .bg(if active {
            rgb_to_u32(255, 255, 255)
        } else {
            rgb_to_u32(246, 243, 249)
        })
        .text_color(if active {
            rgb_to_u32(72, 48, 91)
        } else {
            rgb_to_u32(119, 110, 132)
        })
        .child(
            div()
                .size(px(7.))
                .flex_shrink_0()
                .rounded_full()
                .bg(status_color),
        )
        .child(
            div()
                .flex_1()
                .text_sm()
                .whitespace_nowrap()
                .overflow_hidden()
                .child(format!("{}:{}", profile.host, profile.port)),
        )
        .child(
            div()
                .flex_shrink_0()
                .text_xs()
                .text_color(status_color)
                .child(status_text),
        )
        .child(
            div()
                .id(format!("close-workspace-session-{close_id}"))
                .size(px(22.))
                .rounded_md()
                .flex()
                .items_center()
                .justify_center()
                .text_base()
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb_to_u32(126, 117, 138))
                .hover(|style| {
                    style
                        .bg(rgb_to_u32(235, 226, 241))
                        .text_color(rgb_to_u32(81, 55, 101))
                })
                .child("×")
                .on_click({
                    let workspace = workspace.clone();
                    move |_, _, cx| {
                        cx.stop_propagation();
                        workspace.update(cx, |workspace, cx| workspace.close(&close_id, cx));
                    }
                }),
        )
        .on_click(move |_, _, cx| {
            workspace.update(cx, |workspace, cx| workspace.activate(&session_id, cx));
        })
}

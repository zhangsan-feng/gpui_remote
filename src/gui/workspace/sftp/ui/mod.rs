mod local;
mod remote;
mod select_path_dialog;
mod selection;

pub(super) use select_path_dialog::PathTarget;
pub(super) use selection::MultiSelection;

use super::{CancelTransfer, RetryTransfer, SftpSnapshot, SftpView, TransferRecord};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable, h_flex,
    menu::ContextMenuExt,
    scroll::{Scrollbar, ScrollbarAxis, ScrollbarMode},
    v_flex,
};

use crate::component::theme;

const TRANSFER_PANEL_HEIGHT: f32 = 188.;

impl SftpView {
    pub(super) fn render_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(snapshot) = self.selected_snapshot() else {
            return div()
                .size_full()
                .bg(theme::CustomerUiTheme::workspace_background(cx))
                .into_any_element();
        };
        sync_list_state(&self.local_list_state, self.local.entries.len());
        sync_list_state(&self.remote_list_state, snapshot.entries.len());
        let transfers = self
            .transfers
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        sync_list_state(&self.transfer_list_state, transfers.len());
        self.browser(snapshot, transfers, cx).into_any_element()
    }

    fn browser(
        &self,
        snapshot: SftpSnapshot,
        transfers: Vec<TransferRecord>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = theme::CustomerUiTheme::colors(cx);
        v_flex()
            .on_action(cx.listener(Self::delete_local_entry))
            .on_action(cx.listener(Self::delete_remote_entry))
            .on_action(cx.listener(Self::upload_local_entry))
            .on_action(cx.listener(Self::download_remote_entry))
            .on_action(cx.listener(Self::cancel_transfer))
            .on_action(cx.listener(Self::retry_transfer))
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .rounded_lg()
            .border_1()
            .border_color(theme::CustomerUiTheme::border_color(cx))
            .bg(theme::CustomerUiTheme::workspace_background(cx))
            .text_color(colors.workspace_text_color)
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_stretch()
                    .child(self.local_panel(self.local.clone(), cx))
                    .child(self.remote_panel(snapshot, cx)),
            )
            .child(self.transfer_panel(transfers, cx))
    }

    fn path_bar(&self, path: String, target: PathTarget, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let editable_path = path.clone();
        h_flex()
            .flex_1()
            .min_w_0()
            .h(px(30.))
            .px_3()
            .overflow_hidden()
            .rounded_md()
            .border_1()
            .border_color(colors.border)
            .bg(theme::CustomerUiTheme::panel_background(cx))
            .text_xs()
            .child(path)
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    if event.click_count == 2 {
                        this.open_path_dialog(target, editable_path.clone(), cx);
                    }
                }),
            )
    }

    fn file_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        h_flex()
            .h(px(32.))
            .flex_shrink_0()
            .px_3()
            .border_b_1()
            .border_color(colors.border)
            .bg(theme::CustomerUiTheme::panel_background(cx))
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(colors.muted_foreground)
            .child(div().flex_1().min_w_0().child("名称"))
            .child(div().w(px(86.)).child("大小"))
            .child(div().w(px(132.)).child("修改时间"))
    }

    pub(super) fn entry_name(name: String, is_directory: bool, cx: &App) -> impl IntoElement {
        let colors = cx.theme();
        h_flex()
            .flex_1()
            .min_w_0()
            .gap_2()
            .child(
                Icon::new(if is_directory {
                    IconName::FolderOpen
                } else {
                    IconName::File
                })
                .small()
                .text_color(if is_directory {
                    colors.primary
                } else {
                    colors.muted_foreground
                }),
            )
            .child(div().min_w_0().text_sm().child(name).truncate())
    }

    fn transfer_panel(
        &self,
        transfers: Vec<TransferRecord>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme();
        let transfer_count = transfers.len();
        let menu_view = cx.entity();
        let row_view = cx.weak_entity();
        let content = if transfers.is_empty() {
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_1()
                .text_color(colors.muted_foreground)
                .child(div().text_sm().child("暂无传输记录"))
                .child(div().text_xs().child("上传和下载任务会显示在这里"))
                .into_any_element()
        } else {
            list(self.transfer_list_state.clone(), move |index, _, cx| {
                transfers
                    .get(index)
                    .cloned()
                    .map(|record| {
                        Self::transfer_row(record, row_view.clone(), cx).into_any_element()
                    })
                    .unwrap_or_else(|| div().into_any_element())
            })
            .size_full()
            .into_any_element()
        };
        v_flex()
            .id("sftp-transfer-panel")
            .h(px(TRANSFER_PANEL_HEIGHT))
            .flex_shrink_0()
            .border_t_1()
            .border_color(colors.border)
            .child(
                h_flex()
                    .h(px(38.))
                    .flex_shrink_0()
                    .px_3()
                    .justify_between()
                    .border_b_1()
                    .border_color(colors.border)
                    .bg(theme::CustomerUiTheme::panel_background(cx))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("传输记录"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(colors.muted_foreground)
                            .child(format!("{transfer_count} 个任务")),
                    ),
            )
            .child(
                h_flex()
                    .h(px(30.))
                    .flex_shrink_0()
                    .px_3()
                    .border_b_1()
                    .border_color(colors.border)
                    .text_xs()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.muted_foreground)
                    .child(div().w(px(72.)).child("方向"))
                    .child(div().flex_1().min_w_0().child("文件"))
                    .child(div().flex_1().min_w_0().child("目标"))
                    .child(div().w(px(180.)).child("进度"))
                    .child(div().w(px(100.)).child("速度"))
                    .child(div().w(px(86.)).child("状态")),
            )
            .child(
                h_flex()
                    .size_full()
                    .child(div().gap_2().size_full().overflow_hidden().child(content))
                    .child(
                        div().h_full().w(px(12.)).child(
                            Scrollbar::vertical(&self.transfer_list_state)
                                .mode(ScrollbarMode::Always)
                                .axis(ScrollbarAxis::Vertical),
                        ),
                    ),
            )
            .context_menu(move |menu, _, menu_cx| {
                let Some(transfer_id) = menu_view.read(menu_cx).transfer_context_id else {
                    return menu;
                };
                let record = menu_view
                    .read(menu_cx)
                    .transfers
                    .read()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .iter()
                    .find(|record| record.id == transfer_id)
                    .cloned();
                let _ = menu_view.update(menu_cx, |this, _| {
                    this.transfer_context_id = None;
                });
                let Some(record) = record else {
                    return menu;
                };
                let can_cancel = matches!(record.status.as_str(), "等待中" | "扫描中" | "传输中");
                let can_retry = matches!(record.status.as_str(), "失败" | "已取消");
                let menu = if can_cancel {
                    menu.menu("取消", Box::new(CancelTransfer(transfer_id)))
                } else {
                    menu
                };
                if can_retry {
                    menu.menu("重试", Box::new(RetryTransfer(transfer_id)))
                } else {
                    menu
                }
            })
    }

    fn transfer_row(
        record: TransferRecord,
        view: WeakEntity<SftpView>,
        cx: &App,
    ) -> impl IntoElement {
        let colors = cx.theme();
        let progress = record.progress.clamp(0., 1.);
        let transfer_id = record.id;
        let context_view = view;
        h_flex()
            .id(format!("sftp-transfer-row-{transfer_id}"))
            .h(px(38.))
            .flex_shrink_0()
            .px_3()
            .w_full()
            .border_b_1()
            .border_color(colors.border)
            .text_xs()
            .child(div().w(px(72.)).child(record.direction))
            .child(div().flex_1().min_w_0().child(record.name).truncate())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(colors.muted_foreground)
                    .child(record.target)
                    .truncate(),
            )
            .child(
                h_flex()
                    .w(px(180.))
                    .gap_2()
                    .child(
                        div()
                            .w(px(130.))
                            .h(px(6.))
                            .overflow_hidden()
                            .rounded_full()
                            .bg(colors.secondary)
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(progress))
                                    .rounded_full()
                                    .bg(colors.primary),
                            ),
                    )
                    .child(format!("{:.0}%", progress * 100.)),
            )
            .child(
                div()
                    .w(px(100.))
                    .text_color(colors.muted_foreground)
                    .child(Self::format_transfer_speed(record.speed)),
            )
            .child(div().w(px(86.)).child(record.status))
            .on_mouse_down(MouseButton::Right, move |_, _, cx| {
                let _ = context_view.update(cx, |this, cx| {
                    this.transfer_context_id = Some(transfer_id);
                    cx.notify();
                });
            })
    }

    fn format_transfer_speed(speed: u64) -> String {
        if speed == 0 {
            return "—".to_owned();
        }
        format!("{}/s", Self::format_size(speed))
    }

    fn error_bar(&self, error: String, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        div()
            .flex_shrink_0()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(colors.border)
            .bg(colors.danger)
            .text_xs()
            .text_color(colors.danger_foreground)
            .child(error)
    }

    fn empty_directory(&self, message: &'static str, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .text_xs()
            .text_color(colors.muted_foreground)
            .child(message)
    }

    fn status_view(
        &self,
        title: &'static str,
        message: Option<&str>,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme();
        v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_3()
            .child(
                Icon::new(IconName::FolderOpen)
                    .size(px(36.))
                    .text_color(colors.primary),
            )
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(title),
            )
            .when_some(message.map(str::to_owned), |this, message| {
                this.child(
                    div()
                        .max_w(px(480.))
                        .text_xs()
                        .text_center()
                        .text_color(colors.muted_foreground)
                        .child(message),
                )
            })
    }
}

fn sync_list_state(state: &ListState, item_count: usize) {
    if state.item_count() != item_count {
        state.reset_with_uniform_height(item_count, px(38.));
    }
}

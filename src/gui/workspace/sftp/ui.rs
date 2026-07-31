use super::{
    DragPreviewItem, LocalEntry, LocalSnapshot, SftpEntry, SftpSnapshot, SftpStatus, SftpView,
    TransferRecord,
};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::scroll::{Scrollbar, ScrollbarAxis, ScrollbarShow};
use gpui_component::{
    ActiveTheme, Disableable, Icon, IconName, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};

use crate::component::theme;

const TRANSFER_PANEL_HEIGHT: f32 = 188.;

impl SftpView {
    pub(super) fn render_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let Some(snapshot) = self.selected_snapshot() else {
            return div()
                .size_full()
                .bg(theme::styles(cx).panel)
                .into_any_element();
        };
        sync_list_state(&self.local_list_state, self.local.entries.len());
        sync_list_state(&self.remote_list_state, snapshot.entries.len());
        sync_list_state(&self.transfer_list_state, self.transfers.len());
        self.browser(snapshot, cx).into_any_element()
    }

    fn browser(&self, snapshot: SftpSnapshot, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        v_flex()
            .size_full()
            .min_h_0()
            .overflow_hidden()
            .rounded_lg()
            .border_1()
            .border_color(colors.border)
            .bg(theme::styles(cx).panel)
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_stretch()
                    .child(self.local_panel(self.local.clone(), cx))
                    .child(self.remote_panel(snapshot, cx)),
            )
            .child(self.transfer_panel(cx))
    }

    fn local_panel(&self, snapshot: LocalSnapshot, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let content = if snapshot.entries.is_empty() && !snapshot.loading {
            self.empty_directory("桌面目录为空", cx).into_any_element()
        } else {
            let entries = snapshot.entries.clone();
            let view = cx.weak_entity();
            list(self.local_list_state.clone(), move |index, _, cx| {
                entries
                    .get(index)
                    .cloned()
                    .map(|entry| Self::local_row(entry, view.clone(), cx))
                    .unwrap_or_else(|| div().into_any_element())
            })
            .size_full()
            .into_any_element()
        };
        v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .border_r_1()
            .border_color(colors.border)
            .child(
                h_flex()
                    .h(px(44.))
                    .flex_shrink_0()
                    .px_2()
                    .border_b_1()
                    .border_color(colors.border)
                    .bg(theme::styles(cx).panel)
                    .child(
                        Button::new("sftp-local-parent")
                            .outline()
                            .small()
                            .label("上级")
                            .on_click(cx.listener(Self::go_local_parent)),
                    )
                    .child(
                        Button::new("sftp-local-refresh")
                            .ghost()
                            .small()
                            .label("刷新")
                            .on_click(cx.listener(Self::refresh_local)),
                    )
                    .child(self.path_bar(snapshot.path.display().to_string(), cx))
                    .when(snapshot.loading, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(colors.muted_foreground)
                                .child("读取中…"),
                        )
                    }),
            )
            .when_some(snapshot.error, |this, error| {
                this.child(self.error_bar(error, cx))
            })
            .child(self.file_header(cx))
            .child(
                h_flex()
                    .size_full()
                    .child(div().gap_2().size_full().overflow_hidden().child(content))
                    .child(
                        div().h_full().w(px(12.)).child(
                            Scrollbar::vertical(&self.local_list_state)
                                .scrollbar_show(ScrollbarShow::Always)
                                .axis(ScrollbarAxis::Vertical),
                        ),
                    ),
            )
    }

    fn remote_panel(&self, snapshot: SftpSnapshot, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let connected = snapshot.status == SftpStatus::Connected;
        let content = if snapshot.entries.is_empty() && !snapshot.loading {
            self.empty_directory("远程目录为空", cx).into_any_element()
        } else {
            let entries = snapshot.entries.clone();
            let view = cx.weak_entity();
            list(self.remote_list_state.clone(), move |index, _, cx| {
                entries
                    .get(index)
                    .cloned()
                    .map(|entry| Self::remote_row(entry, view.clone(), cx))
                    .unwrap_or_else(|| div().into_any_element())
            })
            .size_full()
            .into_any_element()
        };
        v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .child(
                h_flex()
                    .h(px(44.))
                    .flex_shrink_0()
                    .px_3()
                    .gap_2()
                    .border_b_1()
                    .border_color(colors.border)
                    .bg(theme::styles(cx).panel)
                    .child(
                        Button::new("sftp-remote-parent")
                            .outline()
                            .small()
                            .label("上级")
                            .disabled(!connected)
                            .on_click(cx.listener(Self::go_parent)),
                    )
                    .child(
                        Button::new("sftp-remote-refresh")
                            .ghost()
                            .small()
                            .label("刷新")
                            .disabled(!connected)
                            .on_click(cx.listener(Self::refresh)),
                    )
                    .child(self.path_bar(snapshot.path.clone(), cx))
                    .when(snapshot.loading, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(colors.muted_foreground)
                                .child("读取中…"),
                        )
                    }),
            )
            .on_drop(cx.listener(|this, dragged: &DragPreviewItem, _window, cx| {
                println!("成功接收到了拖拽路径: {:?}", dragged.path);
                cx.notify();
            }))
            .when_some(snapshot.error.clone(), |this, error| {
                this.child(self.error_bar(error, cx))
            })
            .when(connected, |this| {
                this.child(self.file_header(cx)).child(
                    h_flex()
                        .size_full()
                        .child(div().gap_2().size_full().overflow_hidden().child(content))
                        .child(
                            div().h_full().w(px(12.)).child(
                                Scrollbar::vertical(&self.remote_list_state)
                                    .scrollbar_show(ScrollbarShow::Always)
                                    .axis(ScrollbarAxis::Vertical),
                            ),
                        ),
                )
            })
            .when(!connected, |this| {
                let (title, message) = match snapshot.status {
                    SftpStatus::Connecting => ("正在连接 SFTP…", None),
                    SftpStatus::Failed => ("SFTP 连接失败", snapshot.error.as_deref()),
                    SftpStatus::Disconnected => ("SFTP 连接已断开", snapshot.error.as_deref()),
                    SftpStatus::Connected => unreachable!(),
                };
                this.child(self.status_view(title, message, cx))
            })
    }

    fn path_bar(&self, path: String, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        h_flex()
            .flex_1()
            .min_w_0()
            .h(px(30.))
            .px_3()
            .overflow_hidden()
            .rounded_md()
            .border_1()
            .border_color(colors.border)
            .bg(theme::styles(cx).panel)
            .text_xs()
            .child(path)
    }

    fn file_header(&self, cx: &Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        h_flex()
            .h(px(32.))
            .flex_shrink_0()
            .px_3()
            .border_b_1()
            .border_color(colors.border)
            .bg(theme::styles(cx).panel)
            .text_xs()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(colors.muted_foreground)
            .child(div().flex_1().min_w_0().child("名称"))
            .child(div().w(px(86.)).child("大小"))
            .child(div().w(px(132.)).child("修改时间"))
    }

    fn local_row(entry: LocalEntry, view: WeakEntity<SftpView>, cx: &mut App) -> AnyElement {
        let colors = cx.theme();
        let path = entry.path.clone();
        let is_directory = entry.is_directory;
        h_flex()
            .id(format!("sftp-local-{}", entry.path.display()))
            .h(px(38.))
            .px_3()
            .gap_2()
            .w_full()
            .border_b_1()
            .border_color(colors.border)
            .hover(|style| style.bg(colors.list_hover))
            .cursor_pointer()
            .child(Self::entry_name(entry.name, is_directory, cx))
            .child(
                div()
                    .w(px(86.))
                    .text_xs()
                    .text_color(colors.muted_foreground)
                    .child(if is_directory {
                        "—".to_owned()
                    } else {
                        Self::format_size(entry.size)
                    }),
            )
            .child(
                div()
                    .w(px(132.))
                    .text_xs()
                    .text_color(colors.muted_foreground)
                    .child(Self::format_local_modified(entry.modified_at)),
            )
            .on_drag(
                DragPreviewItem { path: path.clone() },
                |dragged, _, _, cx| {
                    let path = dragged.path.clone();

                    cx.new(move |_| DragPreviewItem { path })
                },
            )
            .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                if is_directory && event.click_count == 2 {
                    let _ = view.update(cx, |this, cx| {
                        this.open_local_directory(path.clone(), cx);
                    });
                }
            })
            .into_any_element()
    }

    fn remote_row(entry: SftpEntry, view: WeakEntity<SftpView>, cx: &mut App) -> AnyElement {
        let colors = cx.theme();
        let path = entry.path.clone();
        let is_directory = entry.is_directory;
        h_flex()
            .id(format!("sftp-remote-{}", entry.path))
            .h(px(38.))
            .w_full()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(colors.border)
            .hover(|style| style.bg(colors.list_hover))
            .cursor_pointer()
            .child(Self::entry_name(entry.name, is_directory, cx))
            .child(
                div()
                    .w(px(86.))
                    .text_xs()
                    .text_color(colors.muted_foreground)
                    .child(if is_directory {
                        "—".to_owned()
                    } else {
                        Self::format_size(entry.size)
                    }),
            )
            .child(
                div()
                    .w(px(132.))
                    .text_xs()
                    .text_color(colors.muted_foreground)
                    .child(Self::format_modified(entry.modified_at)),
            )
            .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                if is_directory && event.click_count == 2 {
                    let _ = view.update(cx, |this, cx| {
                        this.open_directory(path.clone(), cx);
                    });
                }
            })
            .into_any_element()
    }

    fn entry_name(name: String, is_directory: bool, cx: &App) -> impl IntoElement {
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

    fn transfer_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let content = if self.transfers.is_empty() {
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
            let transfers = self.transfers.clone();
            list(self.transfer_list_state.clone(), move |index, _, cx| {
                transfers
                    .get(index)
                    .cloned()
                    .map(|record| Self::transfer_row(record, cx).into_any_element())
                    .unwrap_or_else(|| div().into_any_element())
            })
            .size_full()
            .into_any_element()
        };
        v_flex()
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
                    .bg(theme::styles(cx).panel)
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
                            .child(format!("{} 个任务", self.transfers.len())),
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
                    .child(div().w(px(86.)).child("状态")),
            )
            .child(
                h_flex()
                    .size_full()
                    .child(div().gap_2().size_full().overflow_hidden().child(content))
                    .child(
                        div().h_full().w(px(12.)).child(
                            Scrollbar::vertical(&self.transfer_list_state)
                                .scrollbar_show(ScrollbarShow::Always)
                                .axis(ScrollbarAxis::Vertical),
                        ),
                    ),
            )
    }

    fn transfer_row(record: TransferRecord, cx: &App) -> impl IntoElement {
        let colors = cx.theme();
        let progress = record.progress.clamp(0., 1.);
        h_flex()
            .h(px(38.))
            .flex_shrink_0()
            .px_3()
            .w_full()
            .border_b_1()
            .border_color(colors.border)
            .text_xs()
            .child(div().w(px(72.)).child(record.direction))
            .child(div().flex_1().min_w_0().child(record.name))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_color(colors.muted_foreground)
                    .child(record.target),
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
            .child(div().w(px(86.)).child(record.status))
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

mod remote;
mod local;

use super::{
    DeleteLocalEntry, DeleteRemoteEntry, DragPreviewLocalToRemoteItem,
    DragPreviewRemoteToLocalItem, LocalEntry, LocalSnapshot, SftpEntry, SftpSnapshot, SftpStatus,
    SftpView, TransferRecord,
};
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::scroll::{Scrollbar, ScrollbarAxis, ScrollbarShow};
use gpui_component::{
    ActiveTheme, Disableable, ElementExt, Icon, IconName, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::ContextMenuExt,
    v_flex,
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
        let colors = cx.theme();
        v_flex()
            .on_action(cx.listener(Self::delete_local_entry))
            .on_action(cx.listener(Self::delete_remote_entry))
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
            .child(self.transfer_panel(transfers, cx))
    }

    fn local_panel(&self, snapshot: LocalSnapshot, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let menu_view = cx.entity();
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
            .on_drop(cx.listener(
                |this, dragged: &DragPreviewRemoteToLocalItem, _window, cx| {
                    this.download_file(
                        dragged.path.clone(),
                        dragged.name.clone(),
                        dragged.size,
                        dragged.is_directory,
                        cx,
                    );
                },
            ))
            .context_menu(move |menu, _, menu_cx| {
                // let Some(path) = menu_view.read(menu_cx).local_context_path.clone() else {
                //     return menu;
                // };
                // let _ = menu_view.update(menu_cx, |this, _| {
                //     this.local_context_path = None;
                // });
                // menu.menu_with_icon("删除", IconName::CircleX, Box::new(DeleteLocalEntry(path)))
                menu
            })
    }

    fn remote_panel(&self, snapshot: SftpSnapshot, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme();
        let menu_view = cx.entity();
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
            .on_drop(cx.listener(
                |this, dragged: &DragPreviewLocalToRemoteItem, _window, cx| {
                    this.upload_file(dragged.path.clone(), cx);
                },
            ))
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
            .context_menu(move |menu, _, menu_cx| {
                // let Some((path, is_directory)) =
                //     menu_view.read(menu_cx).remote_context_entry.clone()
                // else {
                //     return menu;
                // };
                // let _ = menu_view.update(menu_cx, |this, _| {
                //     this.remote_context_entry = None;
                // });
                // menu.menu_with_icon(
                //     "删除",
                //     IconName::CircleX,
                //     Box::new(DeleteRemoteEntry { path, is_directory }),
                // )
                menu
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
        let context_path = path.clone();
        let context_view = view.clone();
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
            .on_mouse_down(MouseButton::Right, move |_, _, cx| {
                let _ = context_view.update(cx, |this, cx| {
                    this.local_context_path = Some(context_path.clone());
                    cx.notify();
                });
            })
            .on_drag(
                DragPreviewLocalToRemoteItem { path: path.clone() },
                |dragged, _, _, cx| {
                    let path = dragged.path.clone();
                    cx.new(move |_| DragPreviewLocalToRemoteItem { path })
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
        let context_path = path.clone();
        let context_view = view.clone();
        let is_directory = entry.is_directory;
        let drag_item = DragPreviewRemoteToLocalItem {
            name: entry.name.clone(),
            path: entry.path.clone(),
            size: entry.size,
            is_directory,
        };
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
            .on_mouse_down(MouseButton::Right, move |_, _, cx| {
                let _ = context_view.update(cx, |this, cx| {
                    this.remote_context_entry = Some((context_path.clone(), is_directory));
                    cx.notify();
                });
            })
            .on_drag(drag_item, |dragged, _, _, cx| {
                let dragged = DragPreviewRemoteToLocalItem {
                    name: dragged.name.clone(),
                    path: dragged.path.clone(),
                    size: dragged.size,
                    is_directory: dragged.is_directory,
                };
                cx.new(move |_| dragged)
            })
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

    fn transfer_panel(
        &self,
        transfers: Vec<TransferRecord>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme();
        let transfer_count = transfers.len();
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

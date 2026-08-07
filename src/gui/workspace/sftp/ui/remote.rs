use super::super::{
    DeleteRemoteEntry, DownloadRemoteEntry, DragPreviewLocalToRemoteItem,
    DragPreviewRemoteToLocalItem, RemoteTransferItem, SftpEntry, SftpSnapshot, SftpStatus,
    SftpView,
};
use super::PathTarget;
use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    ActiveTheme, Disableable, Sizable,
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::ContextMenuExt,
    scroll::{Scrollbar, ScrollbarAxis, ScrollbarShow},
    v_flex,
};

use crate::component::theme;

impl SftpView {
    pub(super) fn remote_panel(
        &self,
        snapshot: SftpSnapshot,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme();
        let menu_view = cx.entity();
        let connected = snapshot.status == SftpStatus::Connected;
        let content = if snapshot.entries.is_empty() && !snapshot.loading {
            self.empty_directory("远程目录为空", cx).into_any_element()
        } else {
            let entries = snapshot.entries.clone();
            let selection = self.remote_selection.clone();
            let selected_items = entries
                .iter()
                .filter(|entry| selection.contains(&entry.path))
                .map(|entry| RemoteTransferItem {
                    path: entry.path.clone(),
                    name: entry.name.clone(),
                    size: entry.size,
                    is_directory: entry.is_directory,
                })
                .collect::<Vec<_>>();
            let view = cx.weak_entity();
            list(self.remote_list_state.clone(), move |index, _, cx| {
                entries
                    .get(index)
                    .cloned()
                    .map(|entry| {
                        let selected = selection.contains(&entry.path);
                        Self::remote_row(entry, view.clone(), selected, selected_items.clone(), cx)
                    })
                    .unwrap_or_else(|| div().into_any_element())
            })
            .size_full()
            .into_any_element()
        };
        v_flex()
            .id("sftp-remote-panel")
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
                    .bg(theme::CustomerUiTheme::panel_background(cx))
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
                    .child(self.path_bar(snapshot.path.clone(), PathTarget::Remote, cx))
                    .when(snapshot.loading, |this| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(colors.muted_foreground)
                                .child("读取中…"),
                        )
                    }),
            )
            .on_drop(
                cx.listener(|this, dragged: &DragPreviewLocalToRemoteItem, _, cx| {
                    for path in &dragged.paths {
                        this.upload_file(path.clone(), cx);
                    }
                }),
            )
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
                let Some((path, is_directory)) =
                    menu_view.read(menu_cx).remote_context_entry.clone()
                else {
                    return menu;
                };
                let _ = menu_view.update(menu_cx, |this, _| {
                    this.remote_context_entry = None;
                });
                let Some(entry) =
                    menu_view
                        .read(menu_cx)
                        .selected_snapshot()
                        .and_then(|snapshot| {
                            snapshot
                                .entries
                                .iter()
                                .find(|entry| entry.path == path)
                                .cloned()
                        })
                else {
                    return menu;
                };
                let current_item = RemoteTransferItem {
                    path: path.clone(),
                    name: entry.name.clone(),
                    size: entry.size,
                    is_directory,
                };
                let items = {
                    let view = menu_view.read(menu_cx);
                    if view.remote_selection.contains(&path) {
                        view.selected_snapshot()
                            .map(|snapshot| {
                                snapshot
                                    .entries
                                    .iter()
                                    .filter(|entry| view.remote_selection.contains(&entry.path))
                                    .map(|entry| RemoteTransferItem {
                                        path: entry.path.clone(),
                                        name: entry.name.clone(),
                                        size: entry.size,
                                        is_directory: entry.is_directory,
                                    })
                                    .collect()
                            })
                            .unwrap_or_else(|| vec![current_item.clone()])
                    } else {
                        vec![current_item]
                    }
                };
                menu.menu("下载", Box::new(DownloadRemoteEntry { items }))
                    .menu("删除", Box::new(DeleteRemoteEntry { path, is_directory }))
            })
    }

    fn remote_row(
        entry: SftpEntry,
        view: WeakEntity<SftpView>,
        selected: bool,
        selected_items: Vec<RemoteTransferItem>,
        cx: &mut App,
    ) -> AnyElement {
        let colors = cx.theme();
        let ui_colors = theme::CustomerUiTheme::colors(cx);
        let path = entry.path.clone();
        let is_directory = entry.is_directory;
        let context_path = path.clone();
        let context_view = view.clone();
        let drag_view = view.clone();
        let click_view = view.clone();
        let release_view = view.clone();
        let release_out_view = view.clone();
        let release_path = path.clone();
        let drag_items = if selected {
            selected_items
        } else {
            vec![RemoteTransferItem {
                path: entry.path.clone(),
                name: entry.name.clone(),
                size: entry.size,
                is_directory,
            }]
        };
        let drag_item = DragPreviewRemoteToLocalItem { items: drag_items };
        h_flex()
            .id(format!("sftp-remote-{}", entry.path))
            .h(px(38.))
            .w_full()
            .gap_2()
            .px_3()
            .border_b_1()
            .border_color(colors.border)
            .hover(|style| style.bg(ui_colors.hover_background))
            .when(selected, |this| this.bg(ui_colors.select_background))
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
            .on_drag(drag_item, move |dragged, _, _, cx| {
                let _ = drag_view.update(cx, |this, _| {
                    this.mark_drag_started();
                });
                let items = dragged.items.clone();
                cx.new(move |_| DragPreviewRemoteToLocalItem { items })
            })
            .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                if event.click_count == 2 {
                    if is_directory && !event.modifiers.control {
                        let _ = click_view.update(cx, |this, cx| {
                            this.open_directory(path.clone(), cx);
                        });
                    }
                } else if event.modifiers.control || !selected {
                    let _ = click_view.update(cx, |this, cx| {
                        this.select_remote_path(path.clone(), event.modifiers.control, cx);
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, move |event, _, cx| {
                let _ = release_view.update(cx, |this, cx| {
                    this.finish_remote_click(
                        release_path.clone(),
                        selected,
                        event.modifiers.control,
                        cx,
                    );
                });
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                let _ = release_out_view.update(cx, |this, _| {
                    this.finish_drag();
                });
            })
            .into_any_element()
    }
}

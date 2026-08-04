use super::super::{
    DeleteLocalEntry, DragPreviewRemoteToLocalItem, LocalEntry, LocalSnapshot, SftpView,
    UploadLocalEntry,
};
use super::PathTarget;
use std::path::PathBuf;

use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    menu::ContextMenuExt,
    scroll::{Scrollbar, ScrollbarAxis, ScrollbarShow},
    v_flex, ActiveTheme, Sizable,
};

use crate::component::theme;

impl SftpView {
    pub(super) fn local_panel(
        &self,
        snapshot: LocalSnapshot,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let colors = cx.theme();
        let menu_view = cx.entity();
        let content = if snapshot.entries.is_empty() && !snapshot.loading {
            self.empty_directory("桌面目录为空", cx).into_any_element()
        } else {
            let entries = snapshot.entries.clone();
            let selection = self.local_selection.clone();
            let selected_paths = selection.values();
            let view = cx.weak_entity();
            list(self.local_list_state.clone(), move |index, _, cx| {
                entries
                    .get(index)
                    .cloned()
                    .map(|entry| {
                        let selected = selection.contains(&entry.path);
                        Self::local_row(entry, view.clone(), selected, selected_paths.clone(), cx)
                    })
                    .unwrap_or_else(|| div().into_any_element())
            })
            .size_full()
            .into_any_element()
        };
        v_flex()
            .id("sftp-local-panel")
            .flex_1()
            .min_w_0()
            .min_h_0()
            .border_r_1()
            .border_color(colors.border)
            .child(
                h_flex()
                    .h(px(44.))
                    .flex_shrink_0()
                    .px_3()
                    .gap_2()
                    .border_b_1()
                    .border_color(colors.border)
                    .bg(theme::panel_background(cx))
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
                    .child(self.path_bar(
                        snapshot.path.display().to_string(),
                        PathTarget::Local,
                        cx,
                    ))
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
            .on_drop(
                cx.listener(|this, dragged: &DragPreviewRemoteToLocalItem, _, cx| {
                    for item in &dragged.items {
                        this.download_file(
                            item.path.clone(),
                            item.name.clone(),
                            item.size,
                            item.is_directory,
                            cx,
                        );
                    }
                }),
            )
            .context_menu(move |menu, _, menu_cx| {
                let Some(path) = menu_view.read(menu_cx).local_context_path.clone() else {
                    return menu;
                };
                let _ = menu_view.update(menu_cx, |this, _| {
                    this.local_context_path = None;
                });
                let paths = {
                    let view = menu_view.read(menu_cx);
                    if view.local_selection.contains(&path) {
                        view.local_selection.values()
                    } else {
                        vec![path.clone()]
                    }
                };
                menu.menu("上传", Box::new(UploadLocalEntry(paths)))
                    .menu("删除", Box::new(DeleteLocalEntry(path)))
            })
    }

    fn local_row(
        entry: LocalEntry,
        view: WeakEntity<SftpView>,
        selected: bool,
        selected_paths: Vec<PathBuf>,
        cx: &mut App,
    ) -> AnyElement {
        let colors = cx.theme();
        let styles = theme::styles(cx);
        let path = entry.path.clone();
        let is_directory = entry.is_directory;
        let drag_paths = if selected {
            selected_paths
        } else {
            vec![path.clone()]
        };
        let context_path = path.clone();
        let context_view = view.clone();
        let drag_view = view.clone();
        let click_view = view.clone();
        let release_view = view.clone();
        let release_out_view = view.clone();
        let release_path = path.clone();
        h_flex()
            .id(format!("sftp-local-{}", entry.path.display()))
            .h(px(38.))
            .px_3()
            .gap_2()
            .w_full()
            .border_b_1()
            .border_color(colors.border)
            .hover(|style| style.bg(styles.hover))
            .when(selected, |this| this.bg(styles.selected))
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
                super::super::DragPreviewLocalToRemoteItem { paths: drag_paths },
                move |dragged, _, _, cx| {
                    let _ = drag_view.update(cx, |this, _| {
                        this.mark_drag_started();
                    });
                    let paths = dragged.paths.clone();
                    cx.new(move |_| super::super::DragPreviewLocalToRemoteItem { paths })
                },
            )
            .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                if event.click_count == 2 {
                    if is_directory && !event.modifiers.control {
                        let _ = click_view.update(cx, |this, cx| {
                            this.open_local_directory(path.clone(), cx);
                        });
                    }
                } else if event.modifiers.control || !selected {
                    let _ = click_view.update(cx, |this, cx| {
                        this.select_local_path(path.clone(), event.modifiers.control, cx);
                    });
                }
            })
            .on_mouse_up(MouseButton::Left, move |event, _, cx| {
                let _ = release_view.update(cx, |this, cx| {
                    this.finish_local_click(
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

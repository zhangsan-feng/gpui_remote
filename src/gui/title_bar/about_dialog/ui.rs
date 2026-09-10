use gpui_kit::component::dialog::{Dialog, DialogDescription};
use gpui_kit::*;

pub(super) fn render_about_dialog(dialog: Dialog, _: &mut Window, _: &mut App) -> Dialog {
    dialog.title("关于").w(px(360.)).content(|content, _, _| {
        content
            .child(DialogDescription::new().child(build_time_text(crate::build_info::BUILD_TIME)))
    })
}

fn build_time_text(build_time: &str) -> String {
    format!("编译时间：{build_time}")
}

#[cfg(test)]
mod tests {
    use super::build_time_text;

    #[test]
    fn about_dialog_displays_the_build_time() {
        assert_eq!(
            build_time_text("2026-09-10 12:34:56"),
            "编译时间：2026-09-10 12:34:56"
        );
    }
}

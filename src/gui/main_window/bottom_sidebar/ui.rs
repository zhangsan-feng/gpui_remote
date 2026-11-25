use gpui_kit::{
    assets::IconName,
    component::{ActiveTheme, Icon, Sizable, h_flex, tooltip::Tooltip},
    prelude::FluentBuilder,
    *,
};

use crate::{
    data_context::DATA_CONTEXT,
    domain::{
        host_info::{HostInfoState, HostMemory, RemoteHostInfo},
        terminal::TerminalStatus,
    },
};

pub(super) fn render(cx: &App) -> Div {
    let colors = &cx.theme().colors;
    h_flex()
        .w_full()
        .h(px(32.))
        .flex_none()
        .px_3()
        .gap_4()
        .items_center()
        .overflow_hidden()
        .text_xs()
        .text_color(colors.foreground)
        .border_t_1()
        .border_color(colors.status_bar_border)
        .bg(colors.status_bar)
        .when_some(
            DATA_CONTEXT.selected_ssh_host(),
            |bar, (workspace, state)| {
                let bar = bar.child(metric("主机", IconName::Monitor, workspace.host, cx));
                match (workspace.status, state) {
                    (TerminalStatus::Connected, HostInfoState::Available(info)) => {
                        render_metrics(bar, info, cx)
                    }
                    (TerminalStatus::Connected, HostInfoState::Pending)
                    | (TerminalStatus::Connecting, _) => bar.child(status("正在获取主机信息…", cx)),
                    (TerminalStatus::Connected, HostInfoState::Unavailable) => {
                        bar.child(status("主机信息暂不可用", cx))
                    }
                    (TerminalStatus::Failed | TerminalStatus::Disconnected, _) => {
                        bar.child(status("SSH 未连接", cx))
                    }
                }
            },
        )
}

fn render_metrics(bar: Div, info: RemoteHostInfo, cx: &App) -> Div {
    let cpu = match (info.cpu, info.logical_cpus) {
        (Some(model), Some(cores)) => format!("{model} · {cores} 逻辑核"),
        (None, Some(cores)) => format!("{cores} 逻辑核"),
        (Some(model), None) => model,
        (None, None) => "—".into(),
    };
    let memory = info.memory.map_or_else(|| "—".into(), format_memory);
    let load = info.load.map_or_else(
        || "—".into(),
        |[one, five, fifteen]| format!("{one:.2} / {five:.2} / {fifteen:.2}"),
    );

    bar.child(metric("CPU", IconName::Cpu, cpu, cx))
        .child(metric("内存", IconName::MemoryStick, memory, cx))
        .child(metric("负载 1/5/15 分钟", IconName::Gauge, load, cx))
        .when_some(info.system, |this, system| {
            this.child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_color(cx.theme().colors.muted_foreground)
                    .child(system),
            )
        })
}

fn status(text: &'static str, cx: &App) -> Div {
    div()
        .truncate()
        .text_color(cx.theme().colors.muted_foreground)
        .child(text)
}

fn metric(label: &'static str, icon: IconName, value: String, cx: &App) -> impl IntoElement {
    let muted = cx.theme().colors.muted_foreground;
    let tooltip = format!("{label}  {value}");
    h_flex()
        .id(label)
        .min_w_0()
        .max_w(px(280.))
        .gap_1()
        .items_center()
        .child(
            div()
                .flex_none()
                .text_color(muted)
                .child(Icon::new(icon).xsmall()),
        )
        .child(div().flex_none().text_color(muted).child(label))
        .child(div().min_w_0().truncate().child(value))
        .tooltip(move |window, cx| {
            Tooltip::new(tooltip.clone())
                .max_w(px(460.))
                .build(window, cx)
        })
}

fn format_memory(memory: HostMemory) -> String {
    let used = memory.total_kib - memory.available_kib;
    let (scale, unit) = if memory.total_kib >= 1024 * 1024 {
        (1024. * 1024., "GiB")
    } else {
        (1024., "MiB")
    };
    let used_percent = (used as f64 / memory.total_kib as f64) * 100.;
    format!(
        "{:.1} / {:.1} {unit} ({used_percent:.0}%)",
        used as f64 / scale,
        memory.total_kib as f64 / scale,
    )
}

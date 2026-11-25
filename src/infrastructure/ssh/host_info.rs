use std::time::Duration;

use anyhow::{Context, Result, bail};
use russh::{ChannelMsg, client};
use tokio::time::timeout;

use crate::domain::host_info::{HostMemory, RemoteHostInfo};

const OPEN_TIMEOUT: Duration = Duration::from_secs(3);
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_OUTPUT_BYTES: usize = 16 * 1024;

// Read Linux metrics on a separate SSH channel, including on BusyBox-based hosts.
const HOST_INFO_COMMAND: &str = r#"
export LC_ALL=C
[ -r /proc/cpuinfo ] || exit 1
printf 'gpui-host-info=1\n'
printf 'system=%s\n' "$(uname -sr 2>/dev/null)"
awk -F: '
    /^processor[[:space:]]*:/ { count++ }
    /^model name[[:space:]]*:/ && !model { model=$2 }
    /^(Hardware|Processor)[[:space:]]*:/ && !fallback { fallback=$2 }
    END {
        if (model || fallback) print "cpu=" (model ? model : fallback)
        if (count) print "cores=" count
    }
' /proc/cpuinfo
if [ -r /proc/meminfo ]; then
    awk '
        /^MemTotal:/ { total=$2 }
        /^MemAvailable:/ { available=$2; has_available=1 }
        /^MemFree:/ { free=$2 }
        /^Buffers:/ { buffers=$2 }
        /^Cached:/ { cached=$2 }
        /^SReclaimable:/ { reclaimable=$2 }
        /^Shmem:/ { shared=$2 }
        END {
            if (!has_available) available=free+buffers+cached+reclaimable-shared
            if (total) printf "memory=%.0f %.0f\n", total, (available < 0 ? 0 : available)
        }
    ' /proc/meminfo
fi
if [ -r /proc/loadavg ]; then
    awk 'NR==1 { print "load=" $1 " " $2 " " $3 }' /proc/loadavg
fi
"#;

pub(super) async fn read_host_info<H: client::Handler>(
    session: &client::Handle<H>,
) -> Result<RemoteHostInfo> {
    let mut channel = timeout(OPEN_TIMEOUT, session.channel_open_session())
        .await
        .context("打开主机信息通道超时")?
        .context("打开主机信息通道失败")?;
    let result = timeout(PROBE_TIMEOUT, async {
        channel.exec(true, HOST_INFO_COMMAND).await?;
        let mut output = Vec::new();
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { data } => {
                    if output.len() + data.len() > MAX_OUTPUT_BYTES {
                        bail!("主机信息输出超过长度限制");
                    }
                    output.extend_from_slice(&data);
                }
                ChannelMsg::Failure => bail!("服务器拒绝执行主机信息查询"),
                ChannelMsg::ExitStatus { exit_status } if exit_status != 0 => {
                    bail!("主机信息查询退出状态: {exit_status}");
                }
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }
        parse_host_info(&String::from_utf8_lossy(&output))
    })
    .await;
    let _ = timeout(Duration::from_secs(1), channel.close()).await;
    result.context("采集主机信息超时")?
}

fn parse_host_info(output: &str) -> Result<RemoteHostInfo> {
    let mut lines = output
        .lines()
        .skip_while(|line| *line != "gpui-host-info=1");
    if lines.next().is_none() {
        bail!("远端未返回主机信息");
    }
    let mut info = RemoteHostInfo::default();
    for (key, value) in lines.filter_map(|line| line.split_once('=')) {
        match key {
            "system" => info.system = clean_label(value),
            "cpu" => info.cpu = clean_label(value),
            "cores" => info.logical_cpus = value.parse().ok().filter(|count| *count > 0),
            "memory" => info.memory = parse_memory(value),
            "load" => info.load = parse_load(value),
            _ => {}
        }
    }
    if info == RemoteHostInfo::default() {
        bail!("远端没有可用的主机信息");
    }
    Ok(info)
}

fn clean_label(value: &str) -> Option<String> {
    let value: String = value
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(160)
        .collect();
    (!value.is_empty()).then_some(value)
}

fn parse_memory(value: &str) -> Option<HostMemory> {
    let mut fields = value.split_whitespace();
    let total_kib: u64 = fields.next()?.parse().ok()?;
    let available_kib: u64 = fields.next()?.parse().ok()?;
    (total_kib > 0).then_some(HostMemory {
        total_kib,
        available_kib: available_kib.min(total_kib),
    })
}

fn parse_load(value: &str) -> Option<[f32; 3]> {
    let mut fields = value.split_whitespace();
    let mut loads = [0.0_f32; 3];
    for load in &mut loads {
        *load = fields.next()?.parse().ok()?;
        if !load.is_finite() || *load < 0.0 {
            return None;
        }
    }
    Some(loads)
}

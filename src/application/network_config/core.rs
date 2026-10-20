use super::CONFIG_WRITE;
use crate::{
    application::ApplicationResult, data_context::DATA_CONTEXT,
    domain::network_config::NetworkConfigs, infrastructure::INFRASTRUCTURE,
};

pub(super) async fn initialize() -> ApplicationResult<()> {
    DATA_CONTEXT.set_network_configs(INFRASTRUCTURE.load_network_configs().await?);
    Ok(())
}

pub(super) async fn save(
    proxy: bool,
    id: String,
    name: String,
    config: NetworkConfigs,
    delete: bool,
) -> ApplicationResult<()> {
    let _write = CONFIG_WRITE.lock().await;
    let mut current = DATA_CONTEXT.network_configs();
    if proxy {
        current.proxies.retain(|p| p.id != id);
        if !delete {
            validate_name(&name, current.proxies.iter().map(|p| p.name.as_str()))?;
            let value = config.proxies.into_iter().next().ok_or("代理配置为空")?;
            if value.config.host.trim().is_empty() || value.config.port == 0 {
                return Err("请输入代理主机和有效端口".into());
            }
            current.proxies.push(value);
        }
    } else {
        current.tunnels.retain(|p| p.id != id);
        if !delete {
            validate_name(&name, current.tunnels.iter().map(|p| p.name.as_str()))?;
            let value = config
                .tunnels
                .into_iter()
                .next()
                .ok_or("SSH 隧道配置为空")?;
            if value.config.remote_port == 0
                || value.config.forward_port == 0
                || value.config.forward_address.trim().is_empty()
            {
                return Err("请输入有效的隧道端口和转发地址".into());
            }
            current.tunnels.push(value);
        }
    }
    INFRASTRUCTURE.save_network_configs(current.clone()).await?;
    DATA_CONTEXT.set_network_configs(current);
    log::info!("network_config_saved: proxy={proxy}, delete={delete}, id={id}");
    Ok(())
}

pub(super) fn validate_name<'a>(
    name: &str,
    existing: impl Iterator<Item = &'a str>,
) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("请输入配置名称".into());
    }
    if existing.into_iter().any(|existing| existing == name) {
        return Err("配置名称已存在".into());
    }
    Ok(())
}

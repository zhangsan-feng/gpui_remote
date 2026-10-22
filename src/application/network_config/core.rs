use super::CONFIG_WRITE;
use crate::{
    application::ApplicationResult, data_context::DATA_CONTEXT,
    domain::network_config::NetworkConfigs, infrastructure::INFRASTRUCTURE,
};

pub(super) async fn initialize() -> ApplicationResult<()> {
    let mut configs = INFRASTRUCTURE.load_network_configs().await?;
    update_names(&mut configs);
    DATA_CONTEXT.set_network_configs(configs);
    Ok(())
}

pub(super) async fn save(
    proxy: bool,
    id: String,
    mut config: NetworkConfigs,
    delete: bool,
) -> ApplicationResult<()> {
    let _write = CONFIG_WRITE.lock().await;
    update_names(&mut config);
    let mut current = DATA_CONTEXT.network_configs();
    if proxy {
        current.proxies.retain(|p| p.id != id);
        if !delete {
            let value = config.proxies.into_iter().next().ok_or("代理配置为空")?;
            if value.config.host.trim().is_empty() || value.config.port == 0 {
                return Err("请输入代理主机和有效端口".into());
            }
            if current.proxies.iter().any(|p| p.name == value.name) {
                return Err("相同主机和端口的代理配置已存在".into());
            }
            current.proxies.push(value);
        }
    } else {
        current.tunnels.retain(|p| p.id != id);
        if !delete {
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
            if current.tunnels.iter().any(|p| p.name == value.name) {
                return Err("相同转发参数的 SSH 隧道配置已存在".into());
            }
            current.tunnels.push(value);
        }
    }
    INFRASTRUCTURE.save_network_configs(current.clone()).await?;
    DATA_CONTEXT.set_network_configs(current);
    log::info!("network_config_saved: proxy={proxy}, delete={delete}, id={id}");
    Ok(())
}

fn update_names(configs: &mut NetworkConfigs) {
    for proxy in &mut configs.proxies {
        proxy.name = format!("{}:{}", proxy.config.host, proxy.config.port);
    }
    for tunnel in &mut configs.tunnels {
        tunnel.name = format!(
            "{} → {}:{}",
            tunnel.config.remote_port, tunnel.config.forward_address, tunnel.config.forward_port
        );
    }
}

use super::session_repository::SessionStorageRepository;
use crate::domain::network_config::{NamedConfig, NetworkConfigs};
use anyhow::Result;
use rusqlite::OptionalExtension;

impl SessionStorageRepository {
    pub(crate) fn load_network_configs(&self) -> Result<NetworkConfigs> {
        let stored: Option<String> = {
            let drive = self.lock_drive();
            drive.connection.execute_batch("CREATE TABLE IF NOT EXISTS network_configs (id INTEGER PRIMARY KEY CHECK(id = 1), value TEXT NOT NULL);")?;
            drive
                .connection
                .query_row(
                    "SELECT value FROM network_configs WHERE id = 1",
                    [],
                    |row| row.get(0),
                )
                .optional()?
        };
        if let Some(stored) = stored {
            return Ok(serde_json::from_str(&stored)?);
        }
        let mut configs = NetworkConfigs::default();
        for session in self.list_sessions()? {
            if let Some(config) = session.proxy {
                if !configs.proxies.iter().any(|p| p.config == config) {
                    configs.proxies.push(NamedConfig {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: unique_name(
                            format!("{}:{}", config.host, config.port),
                            configs.proxies.iter().map(|p| p.name.as_str()),
                        ),
                        config,
                    });
                }
            }
            if let Some(config) = session.ssh_reverse_tunnel {
                if !configs.tunnels.iter().any(|p| p.config == config) {
                    configs.tunnels.push(NamedConfig {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: unique_name(
                            format!(
                                "{} → {}:{}",
                                config.remote_port, config.forward_address, config.forward_port
                            ),
                            configs.tunnels.iter().map(|p| p.name.as_str()),
                        ),
                        config,
                    });
                }
            }
        }
        self.save_network_configs(&configs)?;
        Ok(configs)
    }

    pub(crate) fn save_network_configs(&self, configs: &NetworkConfigs) -> Result<()> {
        self.lock_drive().connection.execute("INSERT INTO network_configs (id, value) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET value = excluded.value", [serde_json::to_string(configs)?])?;
        Ok(())
    }
}

fn unique_name<'a>(base: String, names: impl Iterator<Item = &'a str>) -> String {
    let names: Vec<_> = names.collect();
    if !names.contains(&base.as_str()) {
        return base;
    }
    let mut suffix = 2;
    loop {
        let name = format!("{base} ({suffix})");
        if !names.contains(&name.as_str()) {
            return name;
        }
        suffix += 1;
    }
}

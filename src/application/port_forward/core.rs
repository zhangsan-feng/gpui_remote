use std::{
    collections::{HashMap, HashSet},
    net::IpAddr,
    sync::{Arc, Mutex},
};

use super::{PortForwardApplication, PortForwardProjection, PortForwardRuleDraft};
use crate::{
    application::ApplicationResult,
    data_context::DATA_CONTEXT,
    domain::port_forward::{PortForwardRule, PortForwardStatus},
    infrastructure::INFRASTRUCTURE,
};

impl PortForwardApplication {
    pub(super) async fn list_internal(&self) -> ApplicationResult<Vec<PortForwardRule>> {
        let _guard = self.mutations.lock().await;
        load_rules().await
    }

    pub(super) async fn create_internal(
        &self,
        draft: PortForwardRuleDraft,
    ) -> ApplicationResult<PortForwardRule> {
        let mut rule = normalize_draft(draft)?;
        let _guard = self.mutations.lock().await;
        let mut rules = load_rules().await?;
        loop {
            rule.id = uuid::Uuid::new_v4().to_string();
            if !rules.iter().any(|existing| existing.id == rule.id) {
                break;
            }
        }
        rules.push(rule.clone());
        INFRASTRUCTURE.save_port_forward_rules(rules).await?;
        self.reset_status(&rule.id, None);
        Ok(rule)
    }

    pub(super) async fn update_internal(
        &self,
        id: String,
        draft: PortForwardRuleDraft,
    ) -> ApplicationResult<PortForwardRule> {
        let mut replacement = normalize_draft(draft)?;
        let _guard = self.mutations.lock().await;
        let mut rules = load_rules().await?;
        let index = find_rule(&rules, &id)?;
        require_disabled(&rules[index])?;
        replacement.id = id.clone();
        rules[index] = replacement.clone();
        INFRASTRUCTURE.save_port_forward_rules(rules).await?;
        self.reset_status(&id, None);
        Ok(replacement)
    }

    pub(super) async fn delete_internal(&self, id: String) -> ApplicationResult<()> {
        let _guard = self.mutations.lock().await;
        let mut rules = load_rules().await?;
        let index = find_rule(&rules, &id)?;
        require_disabled(&rules[index])?;
        rules.remove(index);
        INFRASTRUCTURE.save_port_forward_rules(rules).await?;
        let mut statuses = self
            .statuses
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        statuses.statuses.remove(&id);
        statuses.runtime_ids.remove(&id);
        DATA_CONTEXT.set_port_forward_statuses(statuses.statuses.clone());
        Ok(())
    }

    pub(super) async fn set_enabled_internal(
        &self,
        id: String,
        enabled: bool,
    ) -> ApplicationResult<()> {
        let _guard = self.mutations.lock().await;
        let mut rules = load_rules().await?;
        let index = find_rule(&rules, &id)?;
        // Always allow disabling a malformed persisted rule so it can be repaired.
        if enabled {
            rules[index] = normalize_rule(rules[index].clone())?;
        }
        rules[index].enabled = enabled;
        let rule = rules[index].clone();
        INFRASTRUCTURE.save_port_forward_rules(rules).await?;
        if enabled {
            self.start(rule).await
        } else {
            INFRASTRUCTURE.stop_port_forward(id.clone()).await?;
            self.publish_runtime();
            self.reset_status(&id, None);
            Ok(())
        }
    }

    pub(super) async fn retry_internal(&self, id: String) -> ApplicationResult<()> {
        let _guard = self.mutations.lock().await;
        let rules = load_rules().await?;
        let rule = rules[find_rule(&rules, &id)?].clone();
        if !rule.enabled {
            return Err("请先启用端口转发规则，再重试".to_owned());
        }
        self.start(rule).await
    }

    pub(super) async fn initialize_internal(&self) -> ApplicationResult<()> {
        let _guard = self.mutations.lock().await;
        let mut subscription = self.subscription.lock().await;
        if subscription.is_some() {
            return Ok(());
        }
        // Subscribe before loading/starting so no listener status change is lost.
        let mut receiver = INFRASTRUCTURE.subscribe_port_forward_status();
        let statuses = self.statuses.clone();
        *subscription = Some(tokio::spawn(async move {
            merge_runtime(&statuses, &mut receiver);
            while receiver.changed().await.is_ok() {
                merge_runtime(&statuses, &mut receiver);
            }
        }));
        let rules = load_rules().await?;
        {
            let mut statuses = self
                .statuses
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            statuses.runtime_ids.clear();
            statuses.statuses = rules
                .iter()
                .map(|rule| {
                    (
                        rule.id.clone(),
                        PortForwardStatus {
                            rule_id: rule.id.clone(),
                            ..Default::default()
                        },
                    )
                })
                .collect();
            DATA_CONTEXT.set_port_forward_statuses(statuses.statuses.clone());
        }
        for rule in rules.into_iter().filter(|rule| rule.enabled) {
            let id = rule.id.clone();
            if let Err(error) = self.start(rule).await {
                log::error!("port_forward_autostart_failed rule_id={id} error={error}");
            }
        }
        Ok(())
    }

    async fn start(&self, rule: PortForwardRule) -> ApplicationResult<()> {
        let id = rule.id.clone();
        let rule = match normalize_rule(rule) {
            Ok(rule) => rule,
            Err(error) => {
                self.reset_status(&id, Some(error.clone()));
                return Err(error);
            }
        };
        {
            let mut projection = self
                .statuses
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            projection.runtime_ids.insert(id.clone());
            projection
                .statuses
                .entry(id.clone())
                .or_insert_with(|| PortForwardStatus {
                    rule_id: id.clone(),
                    ..Default::default()
                });
        }
        let result = INFRASTRUCTURE.start_port_forward(rule).await.map(|_| ());
        self.publish_runtime();
        if let Err(error) = &result {
            log::error!("port_forward_enable_failed rule_id={id} enabled=true error={error}");
        }
        result
    }

    pub(super) async fn shutdown_internal(&self) {
        let _guard = self.mutations.lock().await;
        if let Err(error) = INFRASTRUCTURE.shutdown_port_forwards().await {
            log::warn!("关闭端口转发失败: {error}");
        }
        if let Some(task) = self.subscription.lock().await.take() {
            task.abort();
            let _ = task.await;
        }
        self.publish_runtime();
    }

    fn publish_runtime(&self) {
        let mut receiver = INFRASTRUCTURE.subscribe_port_forward_status();
        merge_runtime(&self.statuses, &mut receiver);
    }

    fn reset_status(&self, id: &str, error: Option<String>) {
        let mut statuses = self
            .statuses
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        statuses.runtime_ids.remove(id);
        statuses.statuses.insert(
            id.to_owned(),
            PortForwardStatus {
                rule_id: id.to_owned(),
                error,
                ..Default::default()
            },
        );
        DATA_CONTEXT.set_port_forward_statuses(statuses.statuses.clone());
    }
}

fn merge_runtime(
    statuses: &Arc<Mutex<PortForwardProjection>>,
    receiver: &mut tokio::sync::watch::Receiver<HashMap<String, PortForwardStatus>>,
) {
    let mut statuses = statuses.lock().unwrap_or_else(|error| error.into_inner());
    // Read the current snapshot after acquiring the projection lock, so a delayed
    // subscriber cannot overwrite a newer immediate publication with old data.
    let runtime = receiver.borrow_and_update();
    // Deleted rules must not reappear when Infrastructure retains an old snapshot.
    for (id, current) in runtime.iter() {
        if statuses.runtime_ids.contains(id) {
            if let Some(status) = statuses.statuses.get_mut(id) {
                *status = current.clone();
            }
        }
    }
    DATA_CONTEXT.set_port_forward_statuses(statuses.statuses.clone());
}

async fn load_rules() -> ApplicationResult<Vec<PortForwardRule>> {
    let rules = INFRASTRUCTURE.load_port_forward_rules().await?;
    let mut ids = HashSet::new();
    for rule in &rules {
        if rule.id.trim().is_empty() || !ids.insert(rule.id.clone()) {
            return Err("端口转发配置包含空或重复规则 ID，请修复配置文件".to_owned());
        }
    }
    Ok(rules)
}

fn find_rule(rules: &[PortForwardRule], id: &str) -> ApplicationResult<usize> {
    rules
        .iter()
        .position(|rule| rule.id == id)
        .ok_or_else(|| format!("端口转发规则不存在: {id}"))
}

fn require_disabled(rule: &PortForwardRule) -> ApplicationResult<()> {
    if rule.enabled {
        Err("请先停用端口转发规则，再编辑或删除".to_owned())
    } else {
        Ok(())
    }
}

fn normalize_rule(rule: PortForwardRule) -> ApplicationResult<PortForwardRule> {
    let mut normalized = normalize_draft(PortForwardRuleDraft {
        listen_host: rule.listen_host,
        listen_port: rule.listen_port.to_string(),
        target_host: rule.target_host,
        target_port: rule.target_port.to_string(),
    })?;
    normalized.id = rule.id;
    normalized.enabled = rule.enabled;
    Ok(normalized)
}

fn normalize_draft(draft: PortForwardRuleDraft) -> ApplicationResult<PortForwardRule> {
    let listen_host = draft
        .listen_host
        .trim()
        .parse::<IpAddr>()
        .map_err(|_| "监听地址必须是 IPv4 或 IPv6 地址".to_owned())?
        .to_string();
    let target_host = draft.target_host.trim().to_owned();
    if target_host.is_empty() {
        return Err("目标主机不能为空".to_owned());
    }
    Ok(PortForwardRule {
        id: String::new(),
        listen_host,
        listen_port: parse_port(&draft.listen_port, "监听")?,
        target_host,
        target_port: parse_port(&draft.target_port, "目标")?,
        enabled: false,
    })
}

fn parse_port(value: &str, label: &str) -> ApplicationResult<u16> {
    value
        .trim()
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(|| format!("{label}端口必须是 1-65535 之间的整数"))
}

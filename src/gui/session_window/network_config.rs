use super::SessionOperationWindow;
use crate::domain::network_config::NamedConfig;
use gpui_kit::component::{IndexPath, searchable_list::SearchableListItem, select::SearchableVec};
use gpui_kit::*;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct ConfigChoice<T: Clone + PartialEq + Eq> {
    pub name: String,
    pub config: Option<T>,
}

impl<T: Clone + PartialEq + Eq + 'static> SearchableListItem for ConfigChoice<T> {
    type Value = Self;
    fn title(&self) -> SharedString {
        self.name.clone().into()
    }
    fn value(&self) -> &Self {
        self
    }
    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div().child(self.name.clone())
    }
}

pub(super) fn choices<T: Clone + PartialEq + Eq>(
    configs: &[NamedConfig<T>],
    selected: Option<T>,
    none: &str,
) -> (Vec<ConfigChoice<T>>, usize) {
    let mut choices = vec![ConfigChoice {
        name: none.into(),
        config: None,
    }];
    choices.extend(configs.iter().map(|p| ConfigChoice {
        name: p.name.clone(),
        config: Some(p.config.clone()),
    }));
    let index = choices
        .iter()
        .position(|p| p.config == selected)
        .unwrap_or_else(|| {
            choices.push(ConfigChoice {
                name: "当前会话配置（已保存参数）".into(),
                config: selected,
            });
            choices.len() - 1
        });
    (choices, index)
}

impl SessionOperationWindow {
    pub(super) fn sync_network_choices(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let configs = crate::data_context::DATA_CONTEXT.network_configs();
        let proxy = self.proxy_select.read(cx).selected_value().cloned();
        let tunnel = self.tunnel_select.read(cx).selected_value().cloned();
        let (proxies, proxy_index) =
            choices(&configs.proxies, proxy.and_then(|p| p.config), "不使用代理");
        let (tunnels, tunnel_index) = choices(
            &configs.tunnels,
            tunnel.and_then(|p| p.config),
            "不使用 SSH 隧道",
        );
        if self.proxy_choices != proxies {
            self.proxy_choices = proxies.clone();
            self.proxy_select.update(cx, |select, cx| {
                select.set_items(SearchableVec::new(proxies), window, cx);
                select.set_selected_index(Some(IndexPath::new(proxy_index)), window, cx);
            });
        }
        if self.tunnel_choices != tunnels {
            self.tunnel_choices = tunnels.clone();
            self.tunnel_select.update(cx, |select, cx| {
                select.set_items(SearchableVec::new(tunnels), window, cx);
                select.set_selected_index(Some(IndexPath::new(tunnel_index)), window, cx);
            });
        }
    }
}

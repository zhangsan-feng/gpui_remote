#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

pub(crate) mod application;
mod build_info;
mod component;
mod data_context;
mod domain;
mod global_state;
mod gui;
mod infrastructure;

use log::info;
use reqwest_client::ReqwestClient;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let _log_guards =
        tokio::task::spawn_blocking(|| infrastructure::logging::initialize("./logs", "%Y-%m-%d"))
            .await
            .expect("initialize logging task failed");
    info!("build time: {}", build_info::BUILD_TIME);

    let application = application::APPLICATION.clone();
    application
        .initialize()
        .await
        .unwrap_or_else(|error| panic!("初始化应用失败: {error}"));
    let theme_settings = application.theme.settings().await;
    let theme_updates = application.theme.settings_updates().await;
    let theme_changed = application.theme.settings_changed().await;
    let shutdown_application = application.clone();

    let http_client = ReqwestClient::user_agent("gpui").unwrap();
    gpui_kit::application()
        .with_http_client(Arc::new(http_client))
        .with_assets(component::assets::MergedAssets::new())
        .run(move |cx| {
            gui::home::open_main_window(
                cx,
                theme_settings.clone(),
                theme_updates.clone(),
                theme_changed.clone(),
            );
        });
    shutdown_application.shutdown().await;
}

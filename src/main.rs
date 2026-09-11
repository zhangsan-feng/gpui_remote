#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

pub(crate) mod application;
mod build_info;
mod component;
mod domain;
mod global_state;
mod gui;
mod infrastructure;

use crate::global_state::{GlobalState, GlobalStateHandle};
use gpui_kit::component::*;
use gpui_kit::*;
use log::info;
use reqwest_client::ReqwestClient;
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing_appender::non_blocking::WorkerGuard;

pub fn logger_init(log_dir: impl AsRef<Path>, date_format: &str) -> [WorkerGuard; 2] {
    let log_dir = log_dir.as_ref();
    std::fs::create_dir_all(log_dir).expect("create log directory failed");

    let log_file = log_dir.join(format!("{}.log", chrono::Local::now().format(date_format)));
    let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
    let file = fern::log_file(&log_file).expect("open log file failed");
    let (file_writer, file_guard) = tracing_appender::non_blocking(file);

    fern::Dispatch::new()
        .format(|out, message, record| {
            let file = record.file().unwrap_or("<unknown>");
            let line = record.line().unwrap_or(0);
            out.finish(format_args!(
                "[{}] [{}] [{}] [{}:{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                file,
                line,
                message
            ))
        })
        // .filter(|metadata| {
        //     metadata.level() == Level::Info && !metadata.target().starts_with("symphonia")
        // })
        .level(log::LevelFilter::Info)
        .level_for("gpui_remote::application", log::LevelFilter::Debug)
        .level_for("gpui_remote::infrastructure", log::LevelFilter::Debug)
        .level_for(
            "gpui_remote::gui::workspace::agent_mcp",
            log::LevelFilter::Debug,
        )
        .level_for("gpui_remote::gui::workspace::ssh", log::LevelFilter::Debug)
        .level_for("gpui_remote::gui::workspace::sftp", log::LevelFilter::Debug)
        // .level_for("gstreamer", log::LevelFilter::Debug)
        // .level(log::LevelFilter::Debug)
        // .level(log::LevelFilter::Trace)
        .chain(fern::Output::writer(Box::new(stdout_writer), "\n"))
        .chain(fern::Output::writer(Box::new(file_writer), "\n"))
        .apply()
        .expect("init logger failed");

    info!("init logger success: {}", log_file.display());
    [stdout_guard, file_guard]
}

#[derive(RustEmbed)]
#[folder = "./src/icon"]
struct AssetFiles;

struct MergedAssets {
    local_directories: Vec<PathBuf>,
    component_assets: gpui_kit::assets::Assets,
}

impl AssetSource for MergedAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        for dir in &self.local_directories {
            let full_path = dir.join(path);
            if full_path.exists() {
                let bytes = std::fs::read(full_path)?;
                return Ok(Some(Cow::Owned(bytes)));
            }
        }

        let clean_path = path.trim_start_matches("icon/").trim_start_matches("/");
        if let Some(file) = AssetFiles::get(clean_path) {
            return Ok(Some(file.data));
        }

        self.component_assets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut all_files = std::collections::HashSet::new();

        for dir in &self.local_directories {
            let full_path = dir.join(path);
            if full_path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(full_path) {
                    for entry in entries.flatten() {
                        if let Some(name) = entry.file_name().to_str() {
                            all_files.insert(name.to_string());
                        }
                    }
                }
            }
        }

        let clean_path = path.trim_start_matches("icon/").trim_start_matches("/");
        for file_path in AssetFiles::iter() {
            if file_path.starts_with(clean_path) {
                all_files.insert(file_path.to_string());
            }
        }

        for file_path in self.component_assets.list(path)? {
            all_files.insert(file_path.to_string());
        }

        Ok(all_files.into_iter().map(SharedString::from).collect())
    }
}

#[tokio::main]
async fn main() {
    let _log_guards = logger_init("./logs", "%Y-%m-%d");
    info!("build time: {}", build_info::BUILD_TIME);

    let http_client = ReqwestClient::user_agent("gpui").unwrap();
    let assets = MergedAssets {
        local_directories: vec![PathBuf::from("/"), PathBuf::from("./src/icon")],
        component_assets: gpui_kit::assets::Assets,
    };

    gpui_kit::application()
        .with_http_client(Arc::new(http_client))
        .with_assets(assets)
        .run(move |cx| {
            let mut window_options = WindowOptions::default();
            let window_size = size(px(1200.), px(700.));
            window_options.window_bounds = Some(WindowBounds::centered(window_size, cx));
            window_options.window_min_size = Some(window_size);
            // window_options.window_background = WindowBackgroundAppearance::Transparent;
            window_options.titlebar = Some(TitlebarOptions {
                title: None,
                appears_transparent: true,
                traffic_light_position: None,
            });

            window_options.window_decorations = Some(WindowDecorations::Client);

            cx.open_window(window_options, |window, app| {
                window.on_window_should_close(app, |window, _| {
                    window.remove_window();
                    false
                });
                gpui_kit::init(app);
                component::theme::init(app);
                window.set_background_appearance(
                    component::theme::CustomerUiTheme::window_background_appearance(app),
                );

                app.new(|cx| {
                    let storage = infrastructure::storage::Storage::new();
                    let infrastructure = infrastructure::new(storage.session.clone());
                    cx.set_global(storage);
                    cx.set_global(infrastructure);
                    info!("infrastructure_global_registered");

                    let application = application::ApplicationContext::new(cx);
                    cx.set_global(application);
                    info!("application_global_registered");

                    let (mcp_bridge, mcp_receiver) = infrastructure::agent_mcp::bridge::new();
                    info!("mcp_bridge_ready");
                    infrastructure::agent_mcp::bridge::start_mcp_bridge(cx, mcp_receiver);
                    infrastructure::agent_mcp::start(mcp_bridge);

                    let global_state = cx.new(|_| GlobalState {});
                    cx.set_global(GlobalStateHandle(global_state));
                    let main_window = cx.new(|cx| gui::home::HomeView::new(window, cx));
                    Root::new(main_window, window, cx)
                })
            })
            .expect("Failed to create app");
        });
}

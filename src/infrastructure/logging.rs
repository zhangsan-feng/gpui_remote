use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;

pub(crate) fn initialize(log_dir: impl AsRef<Path>, date_format: &str) -> [WorkerGuard; 2] {
    let log_dir = log_dir.as_ref();
    std::fs::create_dir_all(log_dir).expect("create log directory failed");

    let application_level = application_log_level();

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
        .level(log::LevelFilter::Warn)
        .level_for("gpui_remote", application_level)
        .chain(fern::Output::writer(Box::new(stdout_writer), "\n"))
        .chain(fern::Output::writer(Box::new(file_writer), "\n"))
        .apply()
        .expect("init logger failed");

    log::info!(
        "logger_initialized: application_level={application_level}, file={}",
        log_file.display()
    );
    [stdout_guard, file_guard]
}

fn application_log_level() -> log::LevelFilter {
    let Ok(level) = std::env::var("GPUI_REMOTE_LOG") else {
        return log::LevelFilter::Info;
    };

    match level.trim().to_ascii_lowercase().as_str() {
        "trace" => log::LevelFilter::Trace,
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "warn" => log::LevelFilter::Warn,
        "error" => log::LevelFilter::Error,
        "off" => log::LevelFilter::Off,
        _ => {
            eprintln!("Invalid GPUI_REMOTE_LOG={level:?}; using info");
            log::LevelFilter::Info
        }
    }
}

use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    Layer,
    filter::Targets,
    fmt::{format::FmtSpan, writer::MakeWriterExt},
    prelude::*,
};

struct LocalTime;

impl tracing_subscriber::fmt::time::FormatTime for LocalTime {
    fn format_time(
        &self,
        writer: &mut tracing_subscriber::fmt::format::Writer<'_>,
    ) -> std::fmt::Result {
        write!(
            writer,
            "{}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        )
    }
}

pub(crate) fn initialize(log_dir: impl AsRef<Path>, date_format: &str) -> [WorkerGuard; 2] {
    let log_dir = log_dir.as_ref();
    std::fs::create_dir_all(log_dir).expect("create log directory failed");

    let application_level = application_log_level();

    let log_file = log_dir.join(format!("{}.log", chrono::Local::now().format(date_format)));
    let (stdout_writer, stdout_guard) = tracing_appender::non_blocking(std::io::stdout());
    let file = fern::log_file(&log_file).expect("open log file failed");
    let (file_writer, file_guard) = tracing_appender::non_blocking(file);

    let tracing_level = application_level
        .to_string()
        .parse::<tracing::level_filters::LevelFilter>()
        .unwrap_or(tracing::level_filters::LevelFilter::INFO);
    let targets = Targets::new()
        .with_default(tracing::Level::WARN)
        .with_target("gpui_remote", tracing_level);
    let tracing_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_timer(LocalTime)
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_writer(file_writer.clone().and(stdout_writer.clone()))
        .with_filter(targets);
    // Install the subscriber directly: fern continues handling existing `log`
    // calls, without a LogTracer bridge that could duplicate or recurse logs.
    tracing::subscriber::set_global_default(tracing_subscriber::registry().with(tracing_layer))
        .expect("init tracing subscriber failed");

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
    initialize_panic_reporting(log_dir);
    [stdout_guard, file_guard]
}

fn initialize_panic_reporting(log_dir: &Path) {
    let path = log_dir.join("panic.log");
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        use std::io::Write;

        // A panic crossing Win32's callback boundary can abort before the async
        // logger drains. This fatal-error path must persist its evidence directly.
        let backtrace = std::backtrace::Backtrace::force_capture();
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let thread = std::thread::current();
            let _ = writeln!(
                file,
                "\n[{}] process={} thread={:?} name={:?} build={}\n{info}\n{backtrace}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                std::process::id(),
                thread.id(),
                thread.name(),
                crate::build_info::BUILD_TIME,
            );
            let _ = file.sync_data();
        }
        tracing::error!(panic = %info, backtrace = %backtrace, "application_panic");
        previous(info);
    }));
    log::info!(
        "panic_reporting_initialized file={}",
        log_dir.join("panic.log").display()
    );
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

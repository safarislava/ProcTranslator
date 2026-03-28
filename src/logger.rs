use std::fs;
use tracing_appender::non_blocking;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, filter::LevelFilter, fmt, prelude::*};

pub fn setup_logger() {
    let _ = fs::create_dir_all("./logs");

    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
    let file_name = format!("app_{}.log", timestamp);

    let file_appender = RollingFileAppender::new(Rotation::NEVER, "./logs", file_name);

    let (non_blocking_file, _guard) = non_blocking(file_appender);
    std::mem::forget(_guard);

    let file_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .parse_lossy("");

    let console_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .parse_lossy("");

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_ansi(true)
                .with_target(false)
                .without_time()
                .compact()
                .with_filter(console_filter),
        )
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_target(false)
                .without_time()
                .with_writer(non_blocking_file)
                .with_filter(file_filter),
        )
        .init();
}

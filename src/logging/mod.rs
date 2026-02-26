mod config;
pub mod targets;

use std::path::Path;

pub use config::{LogRotation, LoggingConfig};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::Layer;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Default)]
pub struct LoggingGuards {
    _technical_guard: Option<WorkerGuard>,
    _decision_guard: Option<WorkerGuard>,
}

pub fn init_logging(config: &LoggingConfig) -> LoggingGuards {
    let terminal_filter = Targets::new().with_default(LevelFilter::WARN);
    let terminal_layer = tracing_subscriber::fmt::layer()
        .compact()
        .with_writer(std::io::stderr)
        .with_filter(terminal_filter);

    if !config.enabled {
        if let Err(err) = tracing_subscriber::registry()
            .with(terminal_layer)
            .try_init()
        {
            eprintln!("logging initialization skipped: {err}");
        }
        return LoggingGuards::default();
    }

    if let Err(err) = std::fs::create_dir_all(Path::new(&config.directory)) {
        eprintln!(
            "failed to create log directory '{}': {err}; falling back to terminal-only logging",
            config.directory
        );

        if let Err(init_err) = tracing_subscriber::registry()
            .with(terminal_layer)
            .try_init()
        {
            eprintln!("logging initialization skipped: {init_err}");
        }
        return LoggingGuards::default();
    }

    let file_level: LevelFilter = config.file_level.into();

    let technical_appender = build_appender(config, &config.technical_file);
    let decision_appender = build_appender(config, &config.decision_file);

    let (technical_writer, technical_guard) = tracing_appender::non_blocking(technical_appender);
    let (decision_writer, decision_guard) = tracing_appender::non_blocking(decision_appender);

    let technical_filter = Targets::new()
        .with_default(file_level)
        .with_target(targets::TRADING_DECISION, LevelFilter::OFF);

    let decision_filter = Targets::new()
        .with_default(LevelFilter::OFF)
        .with_target(targets::TRADING_DECISION, file_level);

    let technical_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(technical_writer)
        .with_filter(technical_filter);

    let decision_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(decision_writer)
        .with_filter(decision_filter);

    if let Err(err) = tracing_subscriber::registry()
        .with(terminal_layer)
        .with(technical_layer)
        .with(decision_layer)
        .try_init()
    {
        eprintln!("logging initialization skipped: {err}");
        return LoggingGuards::default();
    }

    LoggingGuards {
        _technical_guard: Some(technical_guard),
        _decision_guard: Some(decision_guard),
    }
}

fn build_appender(config: &LoggingConfig, file_name: &str) -> RollingFileAppender {
    let rotation = match config.rotation {
        LogRotation::Hourly => Rotation::HOURLY,
        LogRotation::Daily => Rotation::DAILY,
        LogRotation::Never => Rotation::NEVER,
    };

    RollingFileAppender::new(rotation, &config.directory, file_name)
}

use std::{io, str::FromStr};

use serde::Deserialize;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_log(log_file: &String, log_level: Level) -> WorkerGuard {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)
        .expect("failed to open the log file!");
    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(io::stdout)) // stdout layer
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false)) // file layer
        .with(EnvFilter::from_default_env().add_directive(log_level.into()))
        .init();
    tracing::info!("log inited!");

    guard
}

pub fn deserialize_log_level<'de, D>(deserializer: D) -> Result<Level, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Level::from_str(&s).map_err(serde::de::Error::custom)
}

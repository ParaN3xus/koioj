// theoj-judge/src/main.rs

mod config;
mod judge;
mod websocket;

use anyhow::Result;
use std::fs::File;
use theoj_common::utils::init_log;

use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    let config: Config =
        serde_yaml::from_reader(File::open("config.yml").expect("failed to open the config file!"))
            .expect("failed to read the config!");

    let _log_guard = init_log(&config.log_file, config.log_level);

    websocket::run(config).await?;

    Ok(())
}

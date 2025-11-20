// theoj-judge/src/main.rs

mod config;
mod judge;
mod judger;
mod sandbox;
mod websocket;

use clap::{Parser, Subcommand};
use std::fs::File;
use theoj_common::{error::Result, utils::init_log};

use crate::{config::Config, sandbox::install_sandbox};

#[derive(Parser)]
#[command(name = "judge")]
#[command(about = "A judge system", long_about = None)]
struct Cli {
    /// Path to the configuration file
    #[arg(short, long, default_value = "judge_config.yml", global = true)]
    config: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the judge server
    Serve,
    /// Install sandbox environment
    InstallSandbox,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let config: Config =
        serde_yaml::from_reader(File::open(&cli.config).expect("failed to open the config file!"))
            .expect("failed to read the config!");

    let _log_guard = init_log(&config.log_file, config.log_level);

    match cli.command {
        Commands::Serve => {
            websocket::run(config).await?;
        }
        Commands::InstallSandbox => {
            install_sandbox(&config)?;
        }
    }

    Ok(())
}

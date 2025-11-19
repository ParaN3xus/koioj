use std::fs::File;
use theoj_api::{config::Config, start_api};
use theoj_common::{error::Result, utils::init_log};

fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let config: Config =
        serde_yaml::from_reader(File::open("config.yml").expect("failed to open the config file!"))
            .expect("failed to read the config!");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.max_workers)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime");
    let _enter_guard = runtime.enter();

    let _log_guard = init_log(&config.log_file, config.log_level);

    runtime.block_on(start_api(config))
}

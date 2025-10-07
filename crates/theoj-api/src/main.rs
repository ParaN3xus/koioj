use std::fs::File;
use theoj_api::{config::Config, error::Result, init_log, start_api};

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

    let _log_guard = init_log(&config);

    runtime.block_on(start_api(config))
}

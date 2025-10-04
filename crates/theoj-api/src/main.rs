mod config;

use anyhow::Result;
use config::Config;
use sqlx::{
    ConnectOptions, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{
    fs::{File, OpenOptions},
    io::Write,
    str::FromStr,
    sync::Arc,
    time::Instant,
};

pub struct AppState {
    pub config: Arc<Config>,
    pool: PgPool,
    pub started: Instant,
}

impl AppState {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        let opt = PgConnectOptions::from_str(&std::env::var("DATABASE_URL").unwrap())?
            .disable_statement_logging();
        let pool = PgPoolOptions::new()
            .max_connections(32)
            .connect_with(opt)
            .await?;

        Ok(Self {
            config: config,
            pool: pool,
            started: Instant::now(),
        })
    }
}

fn init_log(config: Config) {
    struct MultiWriter<W1: Write, W2: Write> {
        writer1: W1,
        writer2: W2,
    }
    impl<W1: Write, W2: Write> Write for MultiWriter<W1, W2> {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.writer1.write_all(buf)?;
            self.writer2.write_all(buf)?;
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.writer1.flush()?;
            self.writer2.flush()
        }
    }

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(true)
        .open(config.log_file)
        .expect("Failed to open the log file!");

    let multi_writer = MultiWriter {
        writer1: std::io::stdout(),
        writer2: file,
    };

    env_logger::Builder::from_default_env()
        .filter_level(config.log_level.to_level_filter())
        .target(env_logger::Target::Pipe(Box::new(multi_writer)))
        .init();

    log::info!("Log inited!");
}

fn main() -> Result<()> {
    dotenvy::dotenv()?;

    let config: Config =
        serde_yaml::from_reader(File::open("config.yml").expect("Failed to open the config file!"))
            .expect("Failed to read the config!");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(config.max_workers)
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");
    let _enter_guard = runtime.enter();
    init_log(config);

    Ok(())
}

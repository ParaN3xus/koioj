mod auth;
mod config;
mod error;
mod route;

use axum::{
    Extension,
    extract::{DefaultBodyLimit, connect_info::MockConnectInfo},
};
use config::Config;
use error::{Error, Result};
use sqlx::{
    ConnectOptions, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::io;
use std::{
    fs::File,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Instant,
};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::{
    cors::{self, CorsLayer},
    normalize_path::NormalizePathLayer,
    trace::TraceLayer,
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use crate::auth::{generate_strong_password, hash_password};

pub type State = axum::extract::State<Arc<AppState>>;

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

    pub async fn create_admin_account(&self) -> Result<()> {
        let existing_admin: Option<i32> = sqlx::query_scalar!(
            r#"
        SELECT id FROM users WHERE username = 'admin'
        "#
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::msg(format!("check admin account failed: {}", e)))?;
        if let Some(admin_id) = existing_admin {
            if admin_id != 1 {
                tracing::warn!(
                    "admin account already exists but with non-initial id: {}",
                    admin_id
                );
            }
            tracing::info!("admin account already exists, skipping creation");
            return Ok(());
        }

        tracing::warn!("admin account doesn't exist, creating");

        let password_hash = hash_password(self.config.admin_password.clone().unwrap_or({
            let password = generate_strong_password();
            tracing::warn!("admin password doesn't exist in the given config, using {password}");
            password
        }))?;

        let _user_id: i32 = sqlx::query_scalar!(
            r#"
        INSERT INTO users (phone, email, username, user_code, user_type, password, status)
        VALUES ($1, $2, $3, $4, 'admin', $5, 'active')
        RETURNING id
        "#,
            "00000000000",
            "admin@admin.admin",
            "admin",
            "000000000000",
            password_hash
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Error::msg(format!("create admin account failed: {}", e)))?;

        Ok(())
    }
}

fn init_log(config: &Config) -> WorkerGuard {
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&config.log_file)
        .expect("failed to open the log file!");
    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(io::stdout)) // stdout layer
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false)) // file layer
        .with(EnvFilter::from_default_env().add_directive(config.log_level.into()))
        .init();
    tracing::info!("log inited!");

    guard
}

async fn start_api(config: Config) -> Result<()> {
    let config = Arc::new(config);
    let state = Arc::new(AppState::new(Arc::clone(&config)).await?);

    state.create_admin_account().await?;

    let app = route::routes(state.clone())
        .layer(
            ServiceBuilder::new()
                .layer(Extension(MockConnectInfo(IpAddr::V4(
                    Ipv4Addr::UNSPECIFIED,
                ))))
                .layer(TraceLayer::new_for_http().make_span_with(
                    |request: &axum::http::Request<_>| {
                        let request_id = Uuid::new_v4();
                        tracing::info_span!(
                            "http_request",
                            method = %request.method(),
                            uri = %request.uri(),
                            request_id = %request_id,
                        )
                    },
                ))
                .layer(
                    CorsLayer::new()
                        .allow_methods(cors::Any)
                        .allow_headers(cors::Any)
                        .allow_origin(cors::Any),
                )
                .layer(DefaultBodyLimit::max(
                    (config.max_file_size_mb * 1024. * 1024.) as usize,
                ))
                .layer(NormalizePathLayer::trim_trailing_slash()),
        )
        .with_state(Arc::clone(&state));

    tracing::info!("listening on {}", config.listen);
    let listener = TcpListener::bind(&config.listen).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}

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

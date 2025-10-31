mod auth;
pub mod config;
pub mod error;
mod models;
mod perm;
pub mod route;

use axum::{
    Extension,
    extract::{DefaultBodyLimit, connect_info::MockConnectInfo},
};
use config::Config;
use error::{Error, Result};
use serde::{Serialize, de::DeserializeOwned};
use sqlx::{
    ConnectOptions, PgPool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
use std::{io, path::PathBuf};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
    sync::Arc,
    time::Instant,
};
use tokio::{fs, net::TcpListener};
use tower::ServiceBuilder;
use tower_http::{
    cors::{self, CorsLayer},
    normalize_path::NormalizePathLayer,
    trace::TraceLayer,
};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use crate::{
    auth::{generate_strong_password, hash_password},
    models::{ProblemContent, SolutionContent, SubmissionCode, TestCaseData},
};

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
        INSERT INTO users (phone, email, username, user_code, user_role, password, status)
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

    fn get_data_path(&self, subdir: &str, id: i32) -> PathBuf {
        PathBuf::from(&self.config.data_dir)
            .join(subdir)
            .join(format!("{}.json", id))
    }

    async fn write_json_data<T: Serialize>(&self, path: PathBuf, data: &T) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| Error::msg(format!("failed to create directory: {}", e)))?;
        }

        let json = serde_json::to_string_pretty(data)
            .map_err(|e| Error::msg(format!("failed to serialize: {}", e)))?;

        fs::write(&path, json)
            .await
            .map_err(|e| Error::msg(format!("failed to write file: {}", e)))?;

        Ok(())
    }

    async fn read_json_data<T: DeserializeOwned>(&self, path: PathBuf) -> Result<T> {
        let json = fs::read_to_string(&path)
            .await
            .map_err(|e| Error::msg(format!("failed to read file: {}", e)))?;

        serde_json::from_str(&json).map_err(|e| Error::msg(format!("failed to deserialize: {}", e)))
    }

    fn get_problem_content_path(&self, problem_id: i32) -> PathBuf {
        self.get_data_path("problems", problem_id)
    }

    fn get_test_case_path(&self, test_case_id: i32) -> PathBuf {
        self.get_data_path("test_cases", test_case_id)
    }

    fn get_solution_content_path(&self, solution_id: i32) -> PathBuf {
        self.get_data_path("solutions", solution_id)
    }

    fn get_submission_code_path(&self, submission_id: i32) -> PathBuf {
        self.get_data_path("submissions", submission_id)
    }

    pub async fn write_problem_content(
        &self,
        problem_id: i32,
        content: &ProblemContent,
    ) -> Result<()> {
        let path = self.get_problem_content_path(problem_id);
        self.write_json_data(path, content).await
    }

    pub async fn read_problem_content(&self, problem_id: i32) -> Result<ProblemContent> {
        let path = self.get_problem_content_path(problem_id);
        self.read_json_data(path).await
    }

    pub async fn write_test_cases(
        &self,
        test_case_id: i32,
        test_case: &TestCaseData,
    ) -> Result<()> {
        let path = self.get_test_case_path(test_case_id);
        self.write_json_data(path, test_case).await
    }

    pub async fn read_test_cases(&self, test_case_id: i32) -> Result<TestCaseData> {
        let path = self.get_test_case_path(test_case_id);
        self.read_json_data(path).await
    }

    pub async fn write_solution_content(
        &self,
        solution_id: i32,
        content: &SolutionContent,
    ) -> Result<()> {
        let path = self.get_solution_content_path(solution_id);
        self.write_json_data(path, content).await
    }

    pub async fn read_solution_content(&self, solution_id: i32) -> Result<SolutionContent> {
        let path = self.get_solution_content_path(solution_id);
        self.read_json_data(path).await
    }

    pub async fn write_submission_code(
        &self,
        submission_id: i32,
        code: &SubmissionCode,
    ) -> Result<()> {
        let path = self.get_submission_code_path(submission_id);
        self.write_json_data(path, code).await
    }

    pub async fn read_submission_code(&self, submission_id: i32) -> Result<SubmissionCode> {
        let path = self.get_submission_code_path(submission_id);
        self.read_json_data(path).await
    }
}

pub fn init_log(config: &Config) -> WorkerGuard {
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

pub async fn start_api(config: Config) -> Result<()> {
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

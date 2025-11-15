use chrono::Duration;
use serde::Deserialize;
use theoj_common::utils::deserialize_log_level;
use tracing::Level;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub listen: String,
    pub max_workers: usize,
    pub log_file: String,
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: Level,
    pub max_connections: u32,
    pub max_file_size_mb: f32,
    pub jwt_secret: String,
    pub jwt_expiry: Duration,
    pub admin_password: Option<String>,
    pub data_dir: String,
}

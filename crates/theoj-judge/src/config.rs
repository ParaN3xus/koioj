// theoj-judge/src/config.rs

use serde::Deserialize;
use theoj_common::utils::deserialize_log_level;
use tracing::Level;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub judge_id: String,
    pub api_url: String,
    pub log_file: String,
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: Level,
}

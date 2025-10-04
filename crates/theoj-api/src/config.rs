use std::str::FromStr;

use serde::Deserialize;
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
}

fn deserialize_log_level<'de, D>(deserializer: D) -> Result<Level, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Level::from_str(&s).map_err(serde::de::Error::custom)
}

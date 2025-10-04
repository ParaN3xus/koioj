use log::Level;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub listen: String,
    pub max_workers: usize,
    pub log_file: String,
    pub log_level: Level,
    pub max_connections: u32,
}

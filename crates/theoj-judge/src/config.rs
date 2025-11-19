// theoj-judge/src/config.rs

use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;
use theoj_common::utils::deserialize_log_level;
use tracing::Level;

#[derive(Clone, Debug, Deserialize)]
pub struct LanguageConfig {
    pub install: Option<Vec<String>>,
    pub source: String,
    pub compile: Option<Vec<String>>,
    pub compiled: String,
    pub run: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub judge_id: String,
    pub api_url: String,
    pub log_file: String,
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: Level,
    pub judger_bin_path: PathBuf,
    pub rootfs_path: PathBuf,
    pub cgroup_base: PathBuf,
    pub languages: HashMap<String, LanguageConfig>,
}

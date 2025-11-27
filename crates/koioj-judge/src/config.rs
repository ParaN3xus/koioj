// koioj-judge/src/config.rs

use std::{collections::HashMap, path::PathBuf};

use serde::Deserialize;
use koioj_common::{judge::Language, utils::deserialize_log_level};
use tracing::Level;

use crate::sandbox::LanguageConfig;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub judge_id: String,
    pub api_url: String,
    pub log_file: String,
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: Level,
    pub private_key_path: String,
    pub judger_bin_path: PathBuf,
    pub rootfs_path: PathBuf,
    pub cgroup_base: PathBuf,
    pub languages: HashMap<Language, LanguageConfig>,
    pub rootfs_base: String,
    pub rootfs_install: Vec<String>,
}

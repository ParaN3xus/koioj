use serde::Deserialize;
use std::fs::{self};
use std::path::Path;
use std::process::Command;
use theoj_common::error::Context;
use theoj_common::{
    bail,
    error::{Error, Result},
};

use crate::config::Config;

#[derive(Clone, Debug, Deserialize)]
pub struct LanguageConfig {
    pub install: Option<Vec<String>>,
    pub source: String,
    pub compile: Option<Vec<String>>,
    pub compiled: String,
    pub run: Vec<String>,
}

const CHROOT_PATH: &str = "/sbin:/bin:/usr/sbin:/usr/bin:/usr/local/sbin:/usr/local/bin";

pub fn install_sandbox(config: &Config) -> Result<()> {
    let output_dir = &config.rootfs_path;

    fs::create_dir_all(output_dir)?;

    tracing::info!("Downloading rootfs...");
    let status = Command::new("curl")
        .args(["-L", "-o", "rootfs.tar.gz", &config.rootfs_base])
        .status()
        .context("Failed to execute curl")?;

    if !status.success() {
        bail!("Download failed");
    }

    tracing::info!("Extracting rootfs...");
    let status = Command::new("tar")
        .args([
            "-x",
            "-f",
            "rootfs.tar.gz",
            "-C",
            output_dir.to_str().unwrap(),
        ])
        .status()
        .context("Failed to execute tar")?;

    if !status.success() {
        bail!("Extraction failed");
    }

    let _ = fs::remove_file("rootfs.tar.gz");

    // copy resolv.conf
    // install programs requires internet
    if Path::new("/etc/resolv.conf").exists() {
        let _ = fs::copy("/etc/resolv.conf", output_dir.join("etc/resolv.conf"));
    }

    tracing::info!("Running installation commands in chroot...");
    for cmd in &config.rootfs_install {
        tracing::info!("Executing: {}", cmd);
        let status = Command::new("chroot")
            .arg(output_dir.to_str().unwrap())
            .arg("/bin/sh")
            .arg("-c")
            .arg(format!("export PATH={} && {}", CHROOT_PATH, cmd))
            .status()
            .context(format!("Failed to execute: {}", cmd))?;

        if !status.success() {
            tracing::error!("Warning: Command failed: {}", cmd);
        }
    }

    for (lang, lang_config) in &config.languages {
        if let Some(install_cmds) = &lang_config.install {
            tracing::info!("Installing {}...", lang);
            for cmd in install_cmds {
                tracing::info!("Executing: {}", cmd);
                let status = Command::new("chroot")
                    .arg(output_dir.to_str().unwrap())
                    .arg("/bin/sh")
                    .arg("-c")
                    .arg(format!("export PATH={} && {}", CHROOT_PATH, cmd))
                    .status()
                    .context(format!("Failed to execute: {}", cmd))?;

                if !status.success() {
                    tracing::error!("Warning: Command failed for {}: {}", lang, cmd);
                }
            }
        }
    }

    tracing::info!("Sandbox installation completed successfully!");
    Ok(())
}

use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use std::fs;
use std::path::PathBuf;

pub enum FilePath {
    Config,
    LocalData,
}

impl FilePath {
    pub fn validated_dir_path(&self) -> Result<PathBuf> {
        static CONFIG_DIR: Lazy<PathBuf> = Lazy::new(|| {
            let mut dir = dirs_next::config_dir().unwrap_or_else(|| PathBuf::from("~/.config/"));
            dir.push(env!("CARGO_PKG_NAME"));
            dir
        });

        static LOCAL_DATA_PATH: Lazy<PathBuf> = Lazy::new(|| {
            let mut dir =
                dirs_next::data_local_dir().unwrap_or_else(|| PathBuf::from("~/.local/share/"));
            dir.push(env!("CARGO_PKG_NAME"));
            dir
        });

        let dir = match self {
            Self::Config => CONFIG_DIR.clone(),
            Self::LocalData => LOCAL_DATA_PATH.clone(),
        };

        if !dir.exists() {
            fs::create_dir_all(&dir).context("dir creation failed")?;
        }

        Ok(dir)
    }
}

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use xdg::BaseDirectories;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub phone: Option<String>,
    pub api_id: Option<i32>,
    pub api_hash: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            phone: None,
            api_id: None,
            api_hash: None,
        }
    }
}

pub struct Paths {
    pub config_dir: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
    pub download_dir: std::path::PathBuf,
}

impl Paths {
    pub fn new() -> Result<Self> {
        let xdg = BaseDirectories::with_prefix("tg");

        // Get the directories - use place_*_file to create them
        let config_dir = xdg
            .place_config_file("")
            .map(|p| p.parent().unwrap().to_path_buf())
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                std::path::PathBuf::from(home).join(".config/tg")
            });

        let data_dir = xdg
            .place_data_file("")
            .map(|p| p.parent().unwrap().to_path_buf())
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                std::path::PathBuf::from(home).join(".local/share/tg")
            });

        // Get download directory (usually ~/Downloads)
        let download_dir = dirs::download_dir().unwrap_or_else(|| {
            std::env::var("HOME")
                .map(|h| std::path::PathBuf::from(h).join("Downloads"))
                .unwrap_or_else(|_| std::path::PathBuf::from("/tmp"))
        });

        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(&download_dir)?;

        Ok(Self {
            config_dir,
            data_dir,
            download_dir,
        })
    }

    pub fn config_file(&self) -> std::path::PathBuf {
        self.config_dir.join("config.toml")
    }
}

pub fn load_config() -> Result<Config> {
    let paths = Paths::new()?;
    let config_path = paths.config_file();

    if config_path.exists() {
        let contents = fs::read_to_string(&config_path).context("Failed to read config file")?;
        toml::from_str(&contents).context("Failed to parse config file")
    } else {
        Ok(Config::default())
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    let paths = Paths::new()?;
    let config_path = paths.config_file();

    let contents = toml::to_string_pretty(config)?;
    let mut file = fs::File::create(&config_path)?;
    file.write_all(contents.as_bytes())?;

    Ok(())
}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub minimize_to_tray: bool,
    pub start_minimized: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            minimize_to_tray: true,
            start_minimized: false,
        }
    }
}

impl AppSettings {
    fn get_config_path() -> Result<PathBuf> {
        let config_dir = directories::ProjectDirs::from("", "", "BeamMP-Manager")
            .ok_or_else(|| anyhow::anyhow!("Failed to determine config directory"))?
            .config_dir()
            .to_path_buf();

        fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("settings.json"))
    }

    pub fn load() -> Self {
        let path = match Self::get_config_path() {
            Ok(p) => p,
            Err(_) => return Self::default(),
        };

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_config_path()?;
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}


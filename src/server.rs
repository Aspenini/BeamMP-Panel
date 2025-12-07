use crate::config::ServerConfig;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntry {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    #[serde(skip)]
    pub loaded_config: Option<ServerConfig>,
    #[serde(skip)]
    pub edited_config: Option<ServerConfig>,
    #[serde(skip)]
    pub config_error: Option<String>,
}

impl ServerEntry {
    pub fn new(path: PathBuf) -> Result<Self> {
        let config_path = path.join("ServerConfig.toml");
        if !config_path.exists() {
            return Err(anyhow!(
                "ServerConfig.toml not found in the selected folder"
            ));
        }

        let id = uuid::Uuid::new_v4().to_string();
        let mut entry = Self {
            id,
            name: path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string(),
            path,
            loaded_config: None,
            edited_config: None,
            config_error: None,
        };

        entry.load_config();

        // Try to use the server name from config
        if let Some(config) = &entry.loaded_config {
            if !config.general.name.is_empty() {
                entry.name = config.general.name.clone();
            }
        }

        Ok(entry)
    }

    pub fn load_config(&mut self) {
        let config_path = self.path.join("ServerConfig.toml");
        match fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str::<ServerConfig>(&contents) {
                Ok(config) => {
                    self.loaded_config = Some(config.clone());
                    self.edited_config = Some(config);
                    self.config_error = None;
                }
                Err(e) => {
                    self.config_error = Some(format!("Parse error: {}", e));
                    self.loaded_config = None;
                    self.edited_config = None;
                }
            },
            Err(e) => {
                self.config_error = Some(format!("Failed to read config: {}", e));
                self.loaded_config = None;
                self.edited_config = None;
            }
        }
    }

    pub fn save_config(&mut self) -> Result<()> {
        if let Some(config) = &self.edited_config {
            let config_str = toml::to_string_pretty(config)?;
            let config_path = self.path.join("ServerConfig.toml");
            fs::write(config_path, config_str)?;
            self.loaded_config = Some(config.clone());
            Ok(())
        } else {
            Err(anyhow!("No config to save"))
        }
    }

    pub fn revert_config(&mut self) {
        if let Some(original) = &self.loaded_config {
            self.edited_config = Some(original.clone());
        }
    }

    pub fn is_config_dirty(&self) -> bool {
        if let (Some(loaded), Some(edited)) = (&self.loaded_config, &self.edited_config) {
            // Simple comparison - in real world you might want a more sophisticated check
            toml::to_string(loaded).ok() != toml::to_string(edited).ok()
        } else {
            false
        }
    }

    pub fn get_resource_folder(&self) -> String {
        self.edited_config
            .as_ref()
            .map(|c| c.general.resource_folder.clone())
            .unwrap_or_else(|| "Resources".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ServerList {
    pub servers: Vec<ServerEntry>,
}

impl ServerList {
    fn get_config_path() -> Result<PathBuf> {
        let config_dir = directories::ProjectDirs::from("", "", "BeamMP-Manager")
            .ok_or_else(|| anyhow!("Failed to determine config directory"))?
            .config_dir()
            .to_path_buf();

        fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("servers.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::get_config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)?;
        let mut list: ServerList = serde_json::from_str(&contents)?;

        // Load configs for all servers
        for server in &mut list.servers {
            server.load_config();
        }

        Ok(list)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_config_path()?;
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }

    pub fn add_server(&mut self, path: PathBuf) -> Result<String> {
        let entry = ServerEntry::new(path)?;
        let name = entry.name.clone();
        self.servers.push(entry);
        Ok(name)
    }

    pub fn remove_server(&mut self, index: usize) {
        if index < self.servers.len() {
            self.servers.remove(index);
        }
    }
}

// Add uuid dependency to Cargo.toml


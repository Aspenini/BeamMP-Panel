use crate::config::ServerConfig;
use crate::util;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = "ServerConfig.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntry {
    pub id: String,
    /// Display name, kept in sync with the config's `Name` (formatting codes stripped).
    pub name: String,
    pub path: PathBuf,
    /// Start this server when the panel opens.
    #[serde(default)]
    pub auto_start: bool,
    /// Start this server again if it exits without being asked to.
    #[serde(default)]
    pub auto_restart: bool,
    #[serde(skip)]
    pub loaded_config: Option<ServerConfig>,
    #[serde(skip)]
    pub edited_config: Option<ServerConfig>,
    #[serde(skip)]
    pub config_error: Option<String>,
}

impl ServerEntry {
    pub fn new(path: PathBuf) -> Result<Self> {
        if !path.join(CONFIG_FILE).is_file() {
            return Err(anyhow!("{CONFIG_FILE} not found in the selected folder"));
        }

        let mut entry = Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string(),
            path,
            auto_start: false,
            auto_restart: false,
            loaded_config: None,
            edited_config: None,
            config_error: None,
        };
        entry.load_config();
        Ok(entry)
    }

    fn config_path(&self) -> PathBuf {
        self.path.join(CONFIG_FILE)
    }

    pub fn load_config(&mut self) {
        let result = fs::read_to_string(self.config_path())
            .map_err(|e| format!("Failed to read config: {e}"))
            .and_then(|contents| {
                ServerConfig::parse(util::strip_bom(&contents))
                    .map_err(|e| format!("Parse error: {e}"))
            });
        match result {
            Ok(config) => {
                self.update_name(&config);
                self.loaded_config = Some(config.clone());
                self.edited_config = Some(config);
                self.config_error = None;
            }
            Err(e) => {
                self.config_error = Some(e);
                self.loaded_config = None;
                self.edited_config = None;
            }
        }
    }

    /// Save edits back to ServerConfig.toml, changing only the edited fields.
    /// Returns whether the display name changed (so the server list needs saving).
    pub fn save_config(&mut self) -> Result<bool> {
        let edited = self
            .edited_config
            .clone()
            .ok_or_else(|| anyhow!("No config to save"))?;
        let original = self.loaded_config.clone().unwrap_or_default();

        let path = self.config_path();
        // Re-read the file so changes made on disk since loading are kept.
        let mut doc = match fs::read_to_string(&path) {
            Ok(text) => util::strip_bom(&text)
                .parse::<toml_edit::DocumentMut>()
                .map_err(|e| anyhow!("{CONFIG_FILE} on disk is not valid TOML: {e}"))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Default::default(),
            Err(e) => return Err(e.into()),
        };
        edited.apply_changes(&original, &mut doc)?;
        util::atomic_write(&path, doc.to_string().as_bytes())?;

        let renamed = self.update_name(&edited);
        self.loaded_config = Some(edited);
        Ok(renamed)
    }

    fn update_name(&mut self, config: &ServerConfig) -> bool {
        let name = util::strip_color_codes(&config.general.name);
        let name = name.trim();
        if !name.is_empty() && name != self.name {
            self.name = name.to_string();
            true
        } else {
            false
        }
    }

    pub fn revert_config(&mut self) {
        if let Some(original) = &self.loaded_config {
            self.edited_config = Some(original.clone());
        }
    }

    pub fn is_config_dirty(&self) -> bool {
        match (&self.loaded_config, &self.edited_config) {
            (Some(loaded), Some(edited)) => loaded != edited,
            _ => false,
        }
    }

    /// The resource folder the server is actually using (the saved config).
    pub fn resource_folder(&self) -> &str {
        self.loaded_config
            .as_ref()
            .map(|c| c.general.resource_folder.as_str())
            .filter(|f| !f.is_empty())
            .unwrap_or("Resources")
    }

    pub fn executable_exists(&self) -> bool {
        self.path.join(crate::process::SERVER_EXE).is_file()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ServerList {
    pub servers: Vec<ServerEntry>,
}

impl ServerList {
    fn path() -> Result<PathBuf> {
        Ok(util::config_dir()?.join("servers.json"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }

        let contents = fs::read_to_string(path)?;
        let mut list: ServerList = serde_json::from_str(util::strip_bom(&contents))?;
        for server in &mut list.servers {
            server.load_config();
        }
        Ok(list)
    }

    pub fn save(&self) -> Result<()> {
        util::atomic_write(
            &Self::path()?,
            serde_json::to_string_pretty(self)?.as_bytes(),
        )
    }

    pub fn get(&self, id: &str) -> Option<&ServerEntry> {
        self.servers.iter().find(|s| s.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut ServerEntry> {
        self.servers.iter_mut().find(|s| s.id == id)
    }

    /// Register a server folder. Returns the new entry's id.
    pub fn add_server(&mut self, path: PathBuf) -> Result<String> {
        if let Some(existing) = self.servers.iter().find(|s| same_path(&s.path, &path)) {
            return Err(anyhow!(
                "This folder is already added as \"{}\"",
                existing.name
            ));
        }
        let entry = ServerEntry::new(path)?;
        let id = entry.id.clone();
        self.servers.push(entry);
        Ok(id)
    }

    pub fn remove_server(&mut self, id: &str) -> Option<ServerEntry> {
        let index = self.servers.iter().position(|s| s.id == id)?;
        Some(self.servers.remove(index))
    }
}

fn same_path(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

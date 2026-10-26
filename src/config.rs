use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Item, TableLike, Value};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    #[serde(rename = "General")]
    pub general: GeneralConfig,
    #[serde(rename = "Misc")]
    pub misc: MiscConfig,
}

// `serde(default)` keeps parsing tolerant: ServerConfig.toml files written by
// older/newer BeamMP server versions may be missing fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    #[serde(rename = "Port")]
    pub port: u16,
    #[serde(rename = "AuthKey")]
    pub auth_key: String,
    #[serde(rename = "AllowGuests")]
    pub allow_guests: bool,
    #[serde(rename = "LogChat")]
    pub log_chat: bool,
    #[serde(rename = "Debug")]
    pub debug: bool,
    #[serde(rename = "IP")]
    pub ip: String,
    #[serde(rename = "Private")]
    pub private: bool,
    #[serde(rename = "InformationPacket")]
    pub information_packet: bool,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Tags")]
    pub tags: String,
    #[serde(rename = "MaxCars")]
    pub max_cars: i32,
    #[serde(rename = "MaxPlayers")]
    pub max_players: i32,
    #[serde(rename = "Map")]
    pub map: String,
    #[serde(rename = "Description")]
    pub description: String,
    #[serde(rename = "ResourceFolder")]
    pub resource_folder: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MiscConfig {
    #[serde(rename = "ImScaredOfUpdates")]
    pub im_scared_of_updates: bool,
    #[serde(rename = "UpdateReminderTime")]
    pub update_reminder_time: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            port: 30814,
            auth_key: String::new(),
            allow_guests: true,
            log_chat: true,
            debug: false,
            ip: "::".to_string(),
            private: true,
            information_packet: true,
            name: "BeamMP Server".to_string(),
            tags: "Freeroam".to_string(),
            max_cars: 1,
            max_players: 8,
            map: "/levels/gridmap_v2/info.json".to_string(),
            description: "BeamMP Default Description".to_string(),
            resource_folder: "Resources".to_string(),
        }
    }
}

impl Default for MiscConfig {
    fn default() -> Self {
        Self {
            im_scared_of_updates: true,
            update_reminder_time: "30s".to_string(),
        }
    }
}

impl ServerConfig {
    pub fn parse(text: &str) -> Result<Self> {
        Ok(toml_edit::de::from_str(text)?)
    }

    /// Write every field that differs from `original` into `doc`, leaving
    /// everything else in the file untouched: comments, formatting, keys this
    /// panel doesn't know about, and fields changed on disk by someone else.
    pub fn apply_changes(&self, original: &ServerConfig, doc: &mut DocumentMut) -> Result<()> {
        let new = toml_edit::ser::to_document(self)?;
        let old = toml_edit::ser::to_document(original)?;

        for (section, new_section) in new.iter() {
            let Some(new_section) = new_section.as_table_like() else {
                continue;
            };
            let old_section = old.get(section).and_then(Item::as_table_like);

            for (key, new_value) in new_section.iter() {
                let Some(new_value) = new_value.as_value() else {
                    continue;
                };
                let unchanged = old_section
                    .and_then(|t| t.get(key))
                    .and_then(Item::as_value)
                    .is_some_and(|old_value| same_value(old_value, new_value));
                if unchanged {
                    continue;
                }

                let target = doc
                    .entry(section)
                    .or_insert(toml_edit::table())
                    .as_table_like_mut()
                    .ok_or_else(|| anyhow!("[{section}] in ServerConfig.toml is not a table"))?;
                set_preserving_decor(target, key, new_value.clone());
            }
        }
        Ok(())
    }
}

fn same_value(a: &Value, b: &Value) -> bool {
    let (mut a, mut b) = (a.clone(), b.clone());
    a.decor_mut().clear();
    b.decor_mut().clear();
    a.to_string() == b.to_string()
}

/// Replace a value but keep its surrounding whitespace and trailing comment.
fn set_preserving_decor(table: &mut dyn TableLike, key: &str, mut value: Value) {
    match table.get_mut(key) {
        Some(Item::Value(old)) => {
            *value.decor_mut() = old.decor().clone();
            *old = value;
        }
        Some(item) => {
            value.decor_mut().clear();
            *item = Item::Value(value);
        }
        None => {
            value.decor_mut().clear();
            table.insert(key, Item::Value(value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"# BeamMP server config
[General]
Port = 30814 # game port
Name = "^l^4Test Server"
AuthKey = ""
FutureOption = "kept"

[Misc]
ImScaredOfUpdates = false

[HTTP]
HTTPServerPort = 8080
"#;

    #[test]
    fn parses_partial_config() {
        let config = ServerConfig::parse(SAMPLE).unwrap();
        assert_eq!(config.general.port, 30814);
        assert_eq!(config.general.name, "^l^4Test Server");
        // Missing fields fall back to defaults
        assert_eq!(config.general.max_players, 8);
        assert!(!config.misc.im_scared_of_updates);
    }

    #[test]
    fn edits_preserve_comments_and_unknown_keys() {
        let original = ServerConfig::parse(SAMPLE).unwrap();
        let mut edited = original.clone();
        edited.general.port = 40000;
        edited.general.max_players = 16;

        let mut doc: DocumentMut = SAMPLE.parse().unwrap();
        edited.apply_changes(&original, &mut doc).unwrap();
        let out = doc.to_string();

        assert!(out.contains("# BeamMP server config"));
        assert!(out.contains("Port = 40000 # game port"), "{out}");
        assert!(out.contains("FutureOption = \"kept\""));
        assert!(out.contains("[HTTP]"));
        assert!(out.contains("MaxPlayers = 16"));
        // Unchanged defaults are not written out
        assert!(!out.contains("MaxCars"));
        assert_eq!(ServerConfig::parse(&out).unwrap(), edited);
    }

    #[test]
    fn edits_merge_with_external_changes() {
        let original = ServerConfig::parse(SAMPLE).unwrap();
        let mut edited = original.clone();
        edited.general.port = 40000;

        // Someone else changed the name on disk after we loaded it
        let on_disk = SAMPLE.replace("^l^4Test Server", "Renamed");
        let mut doc: DocumentMut = on_disk.parse().unwrap();
        edited.apply_changes(&original, &mut doc).unwrap();
        let merged = ServerConfig::parse(&doc.to_string()).unwrap();
        assert_eq!(merged.general.port, 40000);
        assert_eq!(merged.general.name, "Renamed");
    }
}

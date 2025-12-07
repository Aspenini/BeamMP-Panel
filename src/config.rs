use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(rename = "General")]
    pub general: GeneralConfig,
    #[serde(rename = "Misc")]
    pub misc: MiscConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            misc: MiscConfig::default(),
        }
    }
}


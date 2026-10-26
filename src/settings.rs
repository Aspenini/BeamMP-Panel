use crate::util;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn preference(self) -> egui::ThemePreference {
        match self {
            Theme::System => egui::ThemePreference::System,
            Theme::Light => egui::ThemePreference::Light,
            Theme::Dark => egui::ThemePreference::Dark,
        }
    }
}

/// Panel-wide preferences, stored in settings.json next to servers.json.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Closing the window hides it to the system tray instead of quitting.
    pub close_to_tray: bool,
    /// Start hidden in the tray (also enabled by the `--minimized` flag).
    pub start_minimized: bool,
    /// Ask before quitting while servers are running.
    pub confirm_quit: bool,
    pub theme: Theme,
    /// Lines of output kept per server console.
    pub console_max_lines: usize,
    /// Seconds to wait for a graceful `exit` before force-killing a server.
    pub stop_timeout_secs: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            close_to_tray: true,
            start_minimized: false,
            confirm_quit: true,
            theme: Theme::System,
            console_max_lines: 5000,
            stop_timeout_secs: 10,
        }
    }
}

impl Settings {
    pub const CONSOLE_LINES_RANGE: std::ops::RangeInclusive<usize> = 500..=100_000;
    pub const STOP_TIMEOUT_RANGE: std::ops::RangeInclusive<u64> = 2..=120;

    fn path() -> Result<std::path::PathBuf> {
        Ok(util::config_dir()?.join("settings.json"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|path| Ok(fs::read_to_string(path)?))
            .ok()
            .and_then(|contents| serde_json::from_str::<Settings>(util::strip_bom(&contents)).ok())
            .map(Settings::sanitized)
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        util::atomic_write(&Self::path()?, serde_json::to_string_pretty(self)?.as_bytes())
    }

    fn sanitized(mut self) -> Self {
        self.console_max_lines = self.console_max_lines.clamp(
            *Self::CONSOLE_LINES_RANGE.start(),
            *Self::CONSOLE_LINES_RANGE.end(),
        );
        self.stop_timeout_secs = self.stop_timeout_secs.clamp(
            *Self::STOP_TIMEOUT_RANGE.start(),
            *Self::STOP_TIMEOUT_RANGE.end(),
        );
        self
    }

    pub fn stop_timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.stop_timeout_secs)
    }
}

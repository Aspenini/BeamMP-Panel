use anyhow::{Result, anyhow};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Overrides the data directory (portable installs, testing).
pub const CONFIG_DIR_ENV: &str = "BEAMMP_PANEL_CONFIG_DIR";

/// The panel's own config directory (servers.json, settings.json), created on demand.
pub fn config_dir() -> Result<PathBuf> {
    let dir = match std::env::var_os(CONFIG_DIR_ENV) {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => directories::ProjectDirs::from("", "", "BeamMP-Panel")
            .ok_or_else(|| anyhow!("Failed to determine config directory"))?
            .config_dir()
            .to_path_buf(),
    };
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Write a file via a temp file + rename so a crash mid-write never leaves a
/// truncated config behind.
pub fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let file_name = path
        .file_name()
        .ok_or_else(|| anyhow!("Invalid path: {}", path.display()))?;
    let mut tmp_name = file_name.to_os_string();
    tmp_name.push(".tmp");
    let tmp_path = path.with_file_name(tmp_name);

    let mut file = fs::File::create(&tmp_path)?;
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);

    fs::rename(&tmp_path, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp_path);
    })?;
    Ok(())
}

/// Editors like Notepad may save JSON/TOML with a UTF-8 byte order mark.
pub fn strip_bom(s: &str) -> &str {
    s.strip_prefix('\u{feff}').unwrap_or(s)
}

/// Remove BeamMP chat/name formatting codes (`^0`-`^f`, `^l`, `^r`, ...).
pub fn strip_color_codes(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '^' {
            match chars.next() {
                Some(code) if code.is_ascii_alphanumeric() => {}
                Some(other) => {
                    out.push('^');
                    out.push(other);
                }
                None => out.push('^'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Strip ANSI escape sequences (CSI `ESC [ ... final` and two-byte `ESC x`).
pub fn strip_ansi(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains('\x1b') {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        if chars.next_if_eq(&'[').is_some() {
            // Parameters/intermediates until a final byte in '@'..='~'
            for c in chars.by_ref() {
                if ('@'..='~').contains(&c) {
                    break;
                }
            }
        } else {
            chars.next();
        }
    }
    std::borrow::Cow::Owned(out)
}

/// Open a folder in the platform file manager.
pub fn open_in_file_manager(path: &Path) -> Result<()> {
    let program = if cfg!(windows) {
        "explorer"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(program).arg(path).spawn()?;
    Ok(())
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", value, UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_beammp_codes() {
        assert_eq!(strip_color_codes("^l^4My ^rServer"), "My Server");
        assert_eq!(strip_color_codes("100^%"), "100^%");
        assert_eq!(strip_color_codes("trailing^"), "trailing^");
    }

    #[test]
    fn strips_ansi_sequences() {
        assert_eq!(strip_ansi("\x1b[31m[ERROR]\x1b[0m boom"), "[ERROR] boom");
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[test]
    fn formats_sizes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024), "5.0 MB");
    }
}

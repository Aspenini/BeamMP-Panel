use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModEntry {
    pub relative_path: String,
    pub full_path: PathBuf,
    pub enabled: bool,
}

pub fn scan_mods(server_path: &Path, resource_folder: &str) -> Result<Vec<ModEntry>> {
    // Preallocate capacity for better performance
    let mut mods = Vec::with_capacity(128);

    let enabled_root = server_path.join(resource_folder);
    let disabled_root = server_path.join(format!("{}_disabled", resource_folder));

    // Scan enabled mods
    if enabled_root.exists() {
        scan_directory(&enabled_root, &enabled_root, true, &mut mods)?;
    }

    // Scan disabled mods
    if disabled_root.exists() {
        scan_directory(&disabled_root, &disabled_root, false, &mut mods)?;
    }

    // Sort by path for consistent display
    mods.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(mods)
}

fn scan_directory(
    root: &Path,
    current: &Path,
    enabled: bool,
    mods: &mut Vec<ModEntry>,
) -> Result<()> {
    if !current.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let relative_path = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");

            mods.push(ModEntry {
                relative_path,
                full_path: path,
                enabled,
            });
        } else if path.is_dir() {
            scan_directory(root, &path, enabled, mods)?;
        }
    }

    Ok(())
}

pub fn disable_mod(
    server_path: &Path,
    resource_folder: &str,
    relative_path: &str,
) -> Result<()> {
    let source = server_path.join(resource_folder).join(relative_path);
    let target = server_path
        .join(format!("{}_disabled", resource_folder))
        .join(relative_path);

    // Create parent directories if needed
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(source, target)?;
    Ok(())
}

pub fn enable_mod(
    server_path: &Path,
    resource_folder: &str,
    relative_path: &str,
) -> Result<()> {
    let source = server_path
        .join(format!("{}_disabled", resource_folder))
        .join(relative_path);
    let target = server_path.join(resource_folder).join(relative_path);

    // Create parent directories if needed
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(source, target)?;
    Ok(())
}

pub fn delete_mod(path: &Path) -> Result<()> {
    fs::remove_file(path)?;
    Ok(())
}

pub fn add_mod(server_path: &Path, resource_folder: &str, source_path: &Path) -> Result<()> {
    let file_name = source_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;
    
    let target = server_path.join(resource_folder).join(file_name);

    // Create resource folder if it doesn't exist
    let resource_dir = server_path.join(resource_folder);
    if !resource_dir.exists() {
        fs::create_dir_all(&resource_dir)?;
    }

    fs::copy(source_path, target)?;
    Ok(())
}


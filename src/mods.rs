use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModEntry {
    pub relative_path: String,
    pub full_path: PathBuf,
    pub enabled: bool,
}

pub fn scan_server_mods(server_path: &Path, resource_folder: &str) -> Result<Vec<ModEntry>> {
    // Preallocate capacity for better performance
    let mut mods = Vec::with_capacity(128);

    let enabled_root = server_path.join(resource_folder).join("Server");
    let disabled_root = server_path.join(format!("{}_disabled", resource_folder)).join("Server");

    // Scan enabled server mods (folders only)
    if enabled_root.exists() {
        scan_server_folders(&enabled_root, true, &mut mods)?;
    }

    // Scan disabled server mods (folders only)
    if disabled_root.exists() {
        scan_server_folders(&disabled_root, false, &mut mods)?;
    }

    // Sort by path for consistent display
    mods.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(mods)
}

pub fn scan_client_mods(server_path: &Path, resource_folder: &str) -> Result<Vec<ModEntry>> {
    // Preallocate capacity for better performance
    let mut mods = Vec::with_capacity(128);

    let enabled_root = server_path.join(resource_folder).join("Client");
    let disabled_root = server_path.join(format!("{}_disabled", resource_folder)).join("Client");

    // Scan enabled client mods (ZIP files only)
    if enabled_root.exists() {
        scan_client_files(&enabled_root, true, &mut mods)?;
    }

    // Scan disabled client mods (ZIP files only)
    if disabled_root.exists() {
        scan_client_files(&disabled_root, false, &mut mods)?;
    }

    // Filter out mods.json as it's a server resource
    mods.retain(|mod_entry| {
        !mod_entry.relative_path.eq_ignore_ascii_case("mods.json")
    });

    // Sort by path for consistent display
    mods.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    Ok(mods)
}

fn scan_server_folders(
    root: &Path,
    enabled: bool,
    mods: &mut Vec<ModEntry>,
) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }

    // Server mods are folders in the Server directory
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let folder_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            mods.push(ModEntry {
                relative_path: folder_name,
                full_path: path,
                enabled,
            });
        }
    }

    Ok(())
}

fn scan_client_files(
    root: &Path,
    enabled: bool,
    mods: &mut Vec<ModEntry>,
) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }

    // Client mods are .zip files in the Client directory
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            // Only include .zip files
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("zip") {
                    let file_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    mods.push(ModEntry {
                        relative_path: file_name,
                        full_path: path,
                        enabled,
                    });
                }
            }
        }
    }

    Ok(())
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

pub fn disable_server_mod(
    server_path: &Path,
    resource_folder: &str,
    relative_path: &str,
) -> Result<()> {
    let source = server_path.join(resource_folder).join("Server").join(relative_path);
    let target = server_path
        .join(format!("{}_disabled", resource_folder))
        .join("Server")
        .join(relative_path);

    // Create parent directories if needed
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(source, target)?;
    Ok(())
}

pub fn enable_server_mod(
    server_path: &Path,
    resource_folder: &str,
    relative_path: &str,
) -> Result<()> {
    let source = server_path
        .join(format!("{}_disabled", resource_folder))
        .join("Server")
        .join(relative_path);
    let target = server_path.join(resource_folder).join("Server").join(relative_path);

    // Create parent directories if needed
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(source, target)?;
    Ok(())
}

pub fn disable_client_mod(
    server_path: &Path,
    resource_folder: &str,
    relative_path: &str,
) -> Result<()> {
    let source = server_path.join(resource_folder).join("Client").join(relative_path);
    let target = server_path
        .join(format!("{}_disabled", resource_folder))
        .join("Client")
        .join(relative_path);

    // Create parent directories if needed
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(source, target)?;
    Ok(())
}

pub fn enable_client_mod(
    server_path: &Path,
    resource_folder: &str,
    relative_path: &str,
) -> Result<()> {
    let source = server_path
        .join(format!("{}_disabled", resource_folder))
        .join("Client")
        .join(relative_path);
    let target = server_path.join(resource_folder).join("Client").join(relative_path);

    // Create parent directories if needed
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::rename(source, target)?;
    Ok(())
}

pub fn delete_mod(path: &Path) -> Result<()> {
    // Handle both files (client mods) and directories (server mods)
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn add_client_mod(server_path: &Path, resource_folder: &str, source_path: &Path) -> Result<()> {
    let file_name = source_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;
    
    // Validate that it's a ZIP file
    if let Some(ext) = source_path.extension() {
        if !ext.eq_ignore_ascii_case("zip") {
            return Err(anyhow::anyhow!("Only ZIP files are allowed for client mods"));
        }
    } else {
        return Err(anyhow::anyhow!("Only ZIP files are allowed for client mods"));
    }
    
    let client_dir = server_path.join(resource_folder).join("Client");
    let target = client_dir.join(file_name);

    // Create Resources/Client folder if it doesn't exist
    if !client_dir.exists() {
        fs::create_dir_all(&client_dir)?;
    }

    fs::copy(source_path, target)?;
    Ok(())
}


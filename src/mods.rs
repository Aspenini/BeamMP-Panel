use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModEntry {
    pub relative_path: String,
    pub full_path: PathBuf,
    pub enabled: bool,
    pub is_level: bool,
    pub is_vehicle: bool,
}

#[derive(Debug, Clone)]
pub struct ModDetailInfo {
    pub has_levels: bool,
    pub has_vehicles: bool,
    pub level_names: Vec<String>,
    pub vehicle_names: Vec<String>,
    pub total_files: usize,
    pub total_size: u64,
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
                is_level: false, // Server mods are folders, not levels
                is_vehicle: false,
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

                    // Check if this ZIP contains a "level" or "vehicles" folder
                    let (is_level, is_vehicle) = check_zip_content_type(&path);

                    mods.push(ModEntry {
                        relative_path: file_name,
                        full_path: path,
                        enabled,
                        is_level,
                        is_vehicle,
                    });
                }
            }
        }
    }

    Ok(())
}

fn check_zip_content_type(zip_path: &Path) -> (bool, bool) {
    // Try to open and read the ZIP file
    let file = match fs::File::open(zip_path) {
        Ok(f) => f,
        Err(_) => return (false, false),
    };

    let mut archive = match zip::ZipArchive::new(file) {
        Ok(a) => a,
        Err(_) => return (false, false),
    };

    let mut is_level = false;
    let mut is_vehicle = false;

    // Check if any file in the ZIP is inside a "level/levels" or "vehicles/vehicle" folder
    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_lowercase();
            
            // Check if the path contains "level/" or "levels/" folder
            if name.starts_with("level/") || name.starts_with("levels/") 
                || name.contains("/level/") || name.contains("/levels/") {
                is_level = true;
            }
            
            // Check if the path contains "vehicle/" or "vehicles/" folder
            if name.starts_with("vehicle/") || name.starts_with("vehicles/") 
                || name.contains("/vehicle/") || name.contains("/vehicles/") {
                is_vehicle = true;
            }
            
            // Early exit if both found
            if is_level && is_vehicle {
                break;
            }
        }
    }

    (is_level, is_vehicle)
}

pub fn get_mod_details(zip_path: &Path) -> Result<ModDetailInfo> {
    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    
    let mut has_levels = false;
    let mut has_vehicles = false;
    let mut level_folders = std::collections::HashSet::new();
    let mut vehicle_folders = std::collections::HashSet::new();
    let mut total_size: u64 = 0;
    
    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let name = file.name();
        let name_lower = name.to_lowercase();
        total_size += file.size();
        
        // Check for levels and extract level names
        // Structure is typically: levels/LEVELNAME/... or level/LEVELNAME/...
        if name_lower.starts_with("levels/") || name_lower.starts_with("level/") {
            has_levels = true;
            
            // Split the path and get the level name (second part)
            let parts: Vec<&str> = name.split('/').collect();
            if parts.len() >= 2 {
                let level_name = parts[1];
                if !level_name.is_empty() {
                    level_folders.insert(level_name.to_string());
                }
            }
        } else if name_lower.contains("/levels/") || name_lower.contains("/level/") {
            has_levels = true;
            
            // Find the levels folder in the path
            let parts: Vec<&str> = name.split('/').collect();
            for i in 0..parts.len() - 1 {
                let part_lower = parts[i].to_lowercase();
                if part_lower == "levels" || part_lower == "level" {
                    if i + 1 < parts.len() {
                        let level_name = parts[i + 1];
                        if !level_name.is_empty() {
                            level_folders.insert(level_name.to_string());
                        }
                    }
                    break;
                }
            }
        }
        
        // Check for vehicles and extract vehicle names
        // Structure is typically: vehicles/VEHICLENAME/... or vehicle/VEHICLENAME/...
        if name_lower.starts_with("vehicles/") || name_lower.starts_with("vehicle/") {
            has_vehicles = true;
            
            // Split the path and get the vehicle name (second part)
            let parts: Vec<&str> = name.split('/').collect();
            if parts.len() >= 2 {
                let vehicle_name = parts[1];
                if !vehicle_name.is_empty() {
                    vehicle_folders.insert(vehicle_name.to_string());
                }
            }
        } else if name_lower.contains("/vehicles/") || name_lower.contains("/vehicle/") {
            has_vehicles = true;
            
            // Find the vehicles folder in the path
            let parts: Vec<&str> = name.split('/').collect();
            for i in 0..parts.len() - 1 {
                let part_lower = parts[i].to_lowercase();
                if part_lower == "vehicles" || part_lower == "vehicle" {
                    if i + 1 < parts.len() {
                        let vehicle_name = parts[i + 1];
                        if !vehicle_name.is_empty() {
                            vehicle_folders.insert(vehicle_name.to_string());
                        }
                    }
                    break;
                }
            }
        }
    }
    
    let mut level_names: Vec<String> = level_folders.into_iter().collect();
    level_names.sort();
    
    let mut vehicle_names: Vec<String> = vehicle_folders.into_iter().collect();
    vehicle_names.sort();
    
    Ok(ModDetailInfo {
        has_levels,
        has_vehicles,
        level_names,
        vehicle_names,
        total_files: archive.len(),
        total_size,
    })
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


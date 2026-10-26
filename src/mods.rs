use anyhow::{Result, anyhow};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum ModType {
    Server,
    Client,
}

impl ModType {
    /// Subfolder of the resource folder holding this kind of mod.
    pub fn subdir(self) -> &'static str {
        match self {
            ModType::Server => "Server",
            ModType::Client => "Client",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModEntry {
    /// File name (client mods) or folder name (server mods)
    pub name: String,
    /// Lowercased `name`, for filtering without allocating every frame
    pub name_lower: String,
    pub full_path: PathBuf,
    pub enabled: bool,
    pub is_level: bool,
    pub is_vehicle: bool,
    /// Size on disk; `None` for folders
    pub size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ModDetailInfo {
    pub name: String,
    pub level_names: Vec<String>,
    pub vehicle_names: Vec<String>,
    pub total_files: usize,
    pub archive_size: u64,
    /// `None` when the archive doesn't record sizes up front
    pub uncompressed_size: Option<u128>,
}

/// Remembers which zips contain levels/vehicles, so rescans (after
/// enable/disable, refresh, switching tabs) don't reopen unchanged archives.
/// Keyed by name + size + mtime, which survive the enable/disable move.
type ContentKey = (String, u64, Option<SystemTime>);

#[derive(Default)]
pub struct ContentCache(Mutex<HashMap<ContentKey, (bool, bool)>>);

pub fn enabled_root(server_path: &Path, resource_folder: &str, mod_type: ModType) -> PathBuf {
    server_path.join(resource_folder).join(mod_type.subdir())
}

pub fn disabled_root(server_path: &Path, resource_folder: &str, mod_type: ModType) -> PathBuf {
    server_path
        .join(format!("{resource_folder}_disabled"))
        .join(mod_type.subdir())
}

/// List mods of one type, enabled and disabled. Meant to run off the UI thread.
pub fn scan(
    server_path: &Path,
    resource_folder: &str,
    mod_type: ModType,
    cache: &ContentCache,
) -> Result<Vec<ModEntry>> {
    let mut mods = Vec::new();
    for (root, enabled) in [
        (enabled_root(server_path, resource_folder, mod_type), true),
        (disabled_root(server_path, resource_folder, mod_type), false),
    ] {
        let entries = match fs::read_dir(&root) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(anyhow!("Failed to read {}: {e}", root.display())),
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();

            let mod_entry = match mod_type {
                // Server mods are plugin folders
                ModType::Server if file_type.is_dir() => ModEntry {
                    name_lower: name.to_lowercase(),
                    name,
                    full_path: path,
                    enabled,
                    is_level: false,
                    is_vehicle: false,
                    size: None,
                },
                // Client mods are .zip files (mods.json is the server's own resource)
                ModType::Client
                    if file_type.is_file()
                        && has_zip_extension(&path)
                        && !name.eq_ignore_ascii_case("mods.json") =>
                {
                    let metadata = entry.metadata().ok();
                    let size = metadata.as_ref().map_or(0, |m| m.len());
                    let mtime = metadata.and_then(|m| m.modified().ok());
                    let (is_level, is_vehicle) = cache.content_type(&name, size, mtime, &path);
                    ModEntry {
                        name_lower: name.to_lowercase(),
                        name,
                        full_path: path,
                        enabled,
                        is_level,
                        is_vehicle,
                        size: Some(size),
                    }
                }
                _ => continue,
            };
            mods.push(mod_entry);
        }
    }

    mods.sort_by(|a, b| a.name_lower.cmp(&b.name_lower));
    Ok(mods)
}

fn has_zip_extension(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
}

impl ContentCache {
    fn content_type(
        &self,
        name: &str,
        size: u64,
        mtime: Option<SystemTime>,
        path: &Path,
    ) -> (bool, bool) {
        let key = (name.to_string(), size, mtime);
        if let Some(hit) = self.0.lock().ok().and_then(|c| c.get(&key).copied()) {
            return hit;
        }
        let result = zip_content_type(path);
        if let Ok(mut cache) = self.0.lock() {
            cache.insert(key, result);
        }
        result
    }
}

fn open_archive(path: &Path) -> Result<zip::ZipArchive<BufReader<fs::File>>> {
    Ok(zip::ZipArchive::new(BufReader::new(fs::File::open(path)?))?)
}

/// The asset folder (`levels`, `vehicles`) a zip entry lives under, plus the
/// name of the item folder inside it: `levels/italy/info.json` -> (Level, "italy").
fn classify_entry(name: &[u8]) -> Option<(bool, &[u8])> {
    // Some zip tools write Windows separators
    let mut parts = name.split(|&b| b == b'/' || b == b'\\');
    let mut prev = parts.next()?;
    for part in parts {
        let is_level = prev.eq_ignore_ascii_case(b"levels") || prev.eq_ignore_ascii_case(b"level");
        let is_vehicle =
            prev.eq_ignore_ascii_case(b"vehicles") || prev.eq_ignore_ascii_case(b"vehicle");
        if is_level || is_vehicle {
            return Some((is_level, part));
        }
        prev = part;
    }
    None
}

/// Whether a zip contains levels and/or vehicles. Only reads the central
/// directory (entry names), never file data.
fn zip_content_type(path: &Path) -> (bool, bool) {
    let Ok(archive) = open_archive(path) else {
        return (false, false);
    };
    let (mut is_level, mut is_vehicle) = (false, false);
    for name in archive.file_names_raw() {
        match classify_entry(name) {
            Some((true, _)) => is_level = true,
            Some((false, _)) => is_vehicle = true,
            None => {}
        }
        if is_level && is_vehicle {
            break;
        }
    }
    (is_level, is_vehicle)
}

pub fn mod_details(path: &Path) -> Result<ModDetailInfo> {
    let archive = open_archive(path)?;
    let mut levels = BTreeSet::new();
    let mut vehicles = BTreeSet::new();
    let mut total_files = 0;

    for name in archive.file_names_raw() {
        if !name.ends_with(b"/") {
            total_files += 1;
        }
        if let Some((is_level, item)) = classify_entry(name)
            && !item.is_empty()
        {
            let item = String::from_utf8_lossy(item).into_owned();
            if is_level {
                levels.insert(item);
            } else {
                vehicles.insert(item);
            }
        }
    }

    Ok(ModDetailInfo {
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        level_names: levels.into_iter().collect(),
        vehicle_names: vehicles.into_iter().collect(),
        total_files,
        archive_size: fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        uncompressed_size: archive.decompressed_size(),
    })
}

/// Enable or disable a mod by moving it between `<resource_folder>/<subdir>`
/// and `<resource_folder>_disabled/<subdir>`. Returns the mod's new path.
pub fn set_enabled(
    server_path: &Path,
    resource_folder: &str,
    mod_type: ModType,
    name: &str,
    enabled: bool,
) -> Result<PathBuf> {
    let enabled_path = enabled_root(server_path, resource_folder, mod_type).join(name);
    let disabled_path = disabled_root(server_path, resource_folder, mod_type).join(name);
    let (source, target) = if enabled {
        (disabled_path, enabled_path)
    } else {
        (enabled_path, disabled_path)
    };

    // fs::rename silently replaces files on Windows
    if target.exists() {
        return Err(anyhow!(
            "{} already exists in {}",
            name,
            target.parent().unwrap_or(&target).display()
        ));
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::rename(&source, &target)?;
    Ok(target)
}

pub fn delete(path: &Path) -> Result<()> {
    // Files are client mods, directories are server mods
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Copy a .zip into `<resource_folder>/Client`. Meant to run off the UI
/// thread (map packs can be hundreds of MB).
pub fn add_client_mod(server_path: &Path, resource_folder: &str, source: &Path) -> Result<()> {
    if !has_zip_extension(source) {
        return Err(anyhow!("only .zip files can be client mods"));
    }
    let file_name = source
        .file_name()
        .ok_or_else(|| anyhow!("invalid file name"))?;

    let client_dir = enabled_root(server_path, resource_folder, ModType::Client);
    let target = client_dir.join(file_name);
    let disabled = disabled_root(server_path, resource_folder, ModType::Client).join(file_name);
    if target.exists() || disabled.exists() {
        return Err(anyhow!("a mod with this name already exists"));
    }

    fs::create_dir_all(&client_dir)?;
    // Copy under a temporary name so the server never sees a half-written zip
    let partial = client_dir.join(format!("{}.partial", file_name.to_string_lossy()));
    fs::copy(source, &partial)
        .and_then(|_| fs::rename(&partial, &target))
        .inspect_err(|_| {
            let _ = fs::remove_file(&partial);
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::classify_entry;

    #[test]
    fn classifies_zip_entries() {
        assert_eq!(classify_entry(b"levels/italy/info.json"), Some((true, &b"italy"[..])));
        assert_eq!(classify_entry(b"mod/Levels/foo/"), Some((true, &b"foo"[..])));
        assert_eq!(classify_entry(b"vehicles/pickup/a.jbeam"), Some((false, &b"pickup"[..])));
        assert_eq!(classify_entry(b"vehicles\\pickup\\a.jbeam"), Some((false, &b"pickup"[..])));
        assert_eq!(classify_entry(b"levels/"), Some((true, &b""[..])));
        assert_eq!(classify_entry(b"levels"), None);
        assert_eq!(classify_entry(b"lua/ge/extensions/x.lua"), None);
        assert_eq!(classify_entry(b"mylevels/x"), None);
    }
}

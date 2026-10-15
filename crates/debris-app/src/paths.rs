//! Where the app keeps what it remembers.
//!
//! Two directories, deliberately different:
//!
//! * `%APPDATA%\debris` holds state: settings, custom fixers, backups, and any
//!   extension scripts the user installed. It roams with the profile, because it is
//!   the user's.
//! * `%LOCALAPPDATA%\debris` holds the unpacked script library, which is derived
//!   content that can be regenerated and does not deserve to roam. See
//!   [`crate::bundle::script_dir`].
//!
//! Every file here is JSON. Nothing the user can edit is a bespoke format.

use debris_core::{Extension, Settings, Tweak};
use std::collections::HashMap;
use std::path::PathBuf;

/// `%APPDATA%\debris`, or the temp directory if the profile has no roaming app data.
pub fn root() -> PathBuf {
    base("APPDATA")
}

/// `%LOCALAPPDATA%\debris`, for content that can be regenerated and should not roam.
pub fn local_root() -> PathBuf {
    base("LOCALAPPDATA")
}

/// One profile variable, one fallback. Both directories above are the same shape, so
/// they are the same three lines.
fn base(variable: &str) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("debris")
}

pub fn settings_file() -> PathBuf {
    root().join("settings.json")
}

/// Fixers the user wrote. Each file is a catalogue in the same shape as the shipped
/// one, so there is one format to learn and one parser to trust.
pub fn custom_tweaks_dir() -> PathBuf {
    root().join("fixers")
}

/// Downloaded extension scripts, plus the record of what was installed.
pub fn extensions_dir() -> PathBuf {
    root().join("extensions")
}

/// Catalogue bodies, cached so a catalogue opened once still renders offline.
pub fn catalogues_dir() -> PathBuf {
    extensions_dir().join("catalogues")
}

/// `.reg` backups written before a batch of fixers is applied.
pub fn backups_dir() -> PathBuf {
    root().join("backups")
}

fn installed_file() -> PathBuf {
    extensions_dir().join("installed.json")
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

/// Reads the settings file. A missing or malformed file yields the defaults, because
/// settings are a convenience and refusing to start over one is not a trade worth
/// making.
pub fn load_settings() -> Settings {
    std::fs::read_to_string(settings_file())
        .map(|text| Settings::parse(&text))
        .unwrap_or_default()
}

/// Writes the settings file. A failure is reported and swallowed: the session keeps
/// working, and the cost of a lost setting is that the user sets it again.
pub fn save_settings(settings: &Settings) -> std::io::Result<()> {
    write_json(&settings_file(), &settings.to_json())
}

// ---------------------------------------------------------------------------
// Custom fixers
// ---------------------------------------------------------------------------

/// Every custom fixer file, merged with the shipped catalogue's rules already present
/// filtered out by name, so a user file can deliberately replace a shipped rule.
pub fn load_custom_tweaks() -> Vec<Tweak> {
    let dir = custom_tweaks_dir();
    let mut tweaks = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(_) => return tweaks,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        match Tweak::parse_library(&text) {
            Ok(mut parsed) => tweaks.append(&mut parsed),
            // One bad file should not hide the others, but it should say so.
            Err(error) => eprintln!("debris: {}: {error}", path.display()),
        }
    }
    tweaks
}

// ---------------------------------------------------------------------------
// Backups
// ---------------------------------------------------------------------------

/// Writes a `.reg` backup and returns its path.
///
/// Named after the *content* rather than the clock. Two runs that back up the same
/// values produce the same file instead of a foldersworth of near-identical copies,
/// and two runs that back up different values cannot overwrite each other. The file's
/// timestamp says when; this says what.
pub fn save_backup(text: &str) -> std::io::Result<PathBuf> {
    let dir = backups_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("fixers-{:016x}.reg", fnv1a(text.as_bytes())));
    // Registry files are UTF-16, so `regedit` reads the non-ASCII in a value.
    let mut bytes = Vec::with_capacity(text.len() * 2);
    for unit in text.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(&path, bytes)?;
    Ok(path)
}

/// FNV-1a. Not a security primitive, and not used as one: it only has to tell two
/// documents apart, and content-hash a folder name.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

// ---------------------------------------------------------------------------
// Extensions
// ---------------------------------------------------------------------------

/// Where an extension's script lives. `None` when the URL cannot name a file, which
/// is also what makes [`Extension::file_name`] the only path into this directory.
pub fn extension_path(url: &str) -> Option<PathBuf> {
    Extension::file_name(url).map(|file| extensions_dir().join(file))
}

/// The installed extensions, as `url -> version`.
///
/// Pruned on read: an entry whose script is gone is not installed, whatever the record
/// says. That keeps the record honest when someone clears the folder.
pub fn load_installed() -> HashMap<String, String> {
    let text = std::fs::read_to_string(installed_file()).unwrap_or_default();
    let recorded: HashMap<String, String> = serde_json::from_str(&text).unwrap_or_default();
    recorded
        .into_iter()
        .filter(|(url, _)| extension_path(url).is_some_and(|path| path.is_file()))
        .collect()
}

pub fn save_installed(installed: &HashMap<String, String>) -> std::io::Result<()> {
    let text = serde_json::to_string_pretty(installed).unwrap_or_else(|_| "{}".to_string());
    write_json(&installed_file(), &text)
}

/// Writes a downloaded script to its final name, without recording it yet.
///
/// Staged first because the checksum still has to be checked against what landed. A
/// file that fails verification gets [`remove_extension`], which also deletes it, so
/// the folder never holds a script the app refused to vouch for.
pub fn stage_extension(url: &str, body: &str) -> std::io::Result<PathBuf> {
    let Some(path) = extension_path(url) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the address does not name a .ps1 file",
        ));
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, body)?;
    Ok(path)
}

/// Records a staged script as installed.
pub fn record_installed(url: &str, version: &str) -> std::io::Result<()> {
    let mut installed = load_installed();
    installed.insert(url.to_string(), version.to_string());
    save_installed(&installed)
}

/// Deletes an extension's script and forgets it.
pub fn remove_extension(url: &str) -> std::io::Result<()> {
    if let Some(path) = extension_path(url) {
        let _ = std::fs::remove_file(path);
    }
    let mut installed = load_installed();
    installed.remove(url);
    save_installed(&installed)
}

/// The cached body of a catalogue, and where to keep it.
fn catalogue_cache_path(url: &str) -> PathBuf {
    // A hash rather than the URL: catalogue URLs contain characters Windows will not
    // accept in a filename, and `?token=...` would leak into a directory listing.
    catalogues_dir().join(format!("{:016x}.json", fnv1a(url.as_bytes())))
}

pub fn cache_catalogue(url: &str, body: &str) -> std::io::Result<()> {
    let path = catalogue_cache_path(url);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, body)
}

/// What was cached for a catalogue, if anything.
pub fn cached_catalogue(url: &str) -> Option<String> {
    std::fs::read_to_string(catalogue_cache_path(url)).ok()
}

/// Drops a catalogue's cached body along with its source.
pub fn forget_catalogue(url: &str) {
    let _ = std::fs::remove_file(catalogue_cache_path(url));
}

fn write_json(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_urls_never_share_a_catalogue_cache_file() {
        let a = catalogue_cache_path("https://example.com/a.json");
        let b = catalogue_cache_path("https://example.com/b.json");
        assert_ne!(a, b);
        assert_eq!(a.extension().and_then(|e| e.to_str()), Some("json"));
    }

    #[test]
    fn a_script_path_only_comes_from_a_nameable_url() {
        assert!(extension_path("https://x/y/thing.ps1").is_some());
        assert!(extension_path("https://x/y/readme.md").is_none());
        assert!(extension_path("file:///C:/evil.ps1").is_some());
    }
}

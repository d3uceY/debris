//! The shipped data, and where the app finds it.
//!
//! The release is one exe, so nothing the app reads at runtime may be expected to sit
//! beside it. `build.rs` embeds the icon set, the logo, the system catalogue, the
//! presets and the PowerShell library into the binary, and this module is the only
//! thing in the app that knows it.
//!
//! One part of that cannot be served from memory. PowerShell runs a script through
//! `-File`, and the library imports its shared module through `$PSScriptRoot`, so the
//! scripts have to be real files with their directory shape intact. They are unpacked
//! into the local app data directory, once per build; see [`script_dir`].

use debris_core::script::library;
use debris_core::{BloatCatalog, Preset, Result, ScriptMeta, SystemCatalog, Tweak};
use gpui::SharedString;
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/bundle.rs"));

/// The bytes of an embedded asset, by its path under `assets/`.
pub fn asset(path: &str) -> Option<&'static [u8]> {
    ASSETS
        .iter()
        .find(|file| file.path == path)
        .map(|file| file.bytes)
}

/// The names of the entries directly under `dir`. Nothing in the app enumerates assets
/// — the icon set is addressed one path at a time — but `AssetSource` requires it.
pub fn asset_children(dir: &str) -> Vec<SharedString> {
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    let mut names: Vec<&str> = ASSETS
        .iter()
        .filter_map(|file| file.path.strip_prefix(prefix.as_str()))
        .filter_map(|rest| rest.split('/').next())
        .collect();
    names.sort_unstable();
    names.dedup();
    names.into_iter().map(SharedString::from).collect()
}

/// The packages Windows itself needs. A catalogue that will not load is not fatal: it
/// only means nothing is classified as system-critical and the plan stops warning. That
/// is a degradation worth continuing through, but worth saying out loud.
pub fn system_catalog() -> SystemCatalog {
    match text("data/system-apps.txt") {
        Some(text) => SystemCatalog::parse(text),
        None => {
            eprintln!("debris: no system catalogue in this build");
            SystemCatalog::default()
        }
    }
}

/// The presets shipped in the binary, sorted by name. A preset's name is its file stem,
/// so the filename is the identity.
pub fn presets() -> Vec<Preset> {
    let mut presets: Vec<Preset> = ASSETS
        .iter()
        .filter_map(|file| {
            let name = file.path.strip_prefix("data/presets/")?.strip_suffix(".txt")?;
            Some(Preset::parse(name.replace('-', " "), text_of(file)))
        })
        .collect();
    presets.sort_by(|a, b| a.name.cmp(&b.name));
    presets
}

/// The fixers, as shipped. Returns nothing rather than failing when the catalogue is
/// missing or malformed: the app can inventory packages perfectly well without it, and
/// refusing to start over a data file would be worse than a thinner surface.
pub fn fixers() -> Vec<Tweak> {
    let Some(text) = text("data/fixers.json") else {
        eprintln!("debris: no fixer catalogue in this build");
        return Vec::new();
    };
    match Tweak::parse_library(text) {
        Ok(tweaks) => tweaks,
        Err(error) => {
            eprintln!("debris: {error}");
            Vec::new()
        }
    }
}

/// The bloatware catalogue, for naming and describing what the inventory found.
///
/// Empty is a working state: nothing is described, and nothing else changes.
pub fn bloatware() -> BloatCatalog {
    match text("data/bloatware.json") {
        Some(text) => match BloatCatalog::parse(text) {
            Ok(catalog) => catalog,
            Err(error) => {
                eprintln!("debris: {error}");
                BloatCatalog::default()
            }
        },
        None => BloatCatalog::default(),
    }
}

/// The script library, unpacked and parsed.
pub fn scripts() -> Result<Vec<ScriptMeta>> {
    library(script_dir())
}

/// The unpacked script library, written the first time this build asks for it.
///
/// The directory is named after the *content* of the embedded library, which is what
/// makes a deleted script disappear: a build that changes anything unpacks into a fresh
/// empty directory instead of leaving the old file behind. It is also what makes editing
/// a script take effect in a debug build without clearing anything by hand.
pub fn script_dir() -> PathBuf {
    let root = data_root().join("scripts");
    let dir = root.join(bundle_id());
    if dir.is_dir() {
        return dir;
    }

    // Unpacked into a sibling and moved into place, so a half-written directory is never
    // the one the app runs from. The staging name is fixed because a crash mid-write can
    // leave it behind, and the next launch clears it before it starts.
    let staging = root.join("staging");
    let _ = std::fs::remove_dir_all(&staging);
    match unpack(&staging).and_then(|()| std::fs::rename(&staging, &dir)) {
        Ok(()) => prune(&root, &dir),
        Err(error) => eprintln!("debris: could not unpack the script library: {error}"),
    }
    dir
}

/// The identity of the embedded library: the hash of every script's path and bytes.
fn bundle_id() -> String {
    let mut bytes = Vec::new();
    for file in SCRIPTS {
        bytes.extend_from_slice(file.path.as_bytes());
        bytes.extend_from_slice(file.bytes);
    }
    format!("{:016x}", crate::paths::fnv1a(&bytes))
}

/// The local app data directory. Derived content lives here rather than in the
/// roaming profile; see [`crate::paths`].
fn data_root() -> PathBuf {
    crate::paths::local_root()
}

fn unpack(staging: &Path) -> std::io::Result<()> {
    for file in SCRIPTS {
        let path = staging.join(file.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, file.bytes)?;
    }
    Ok(())
}

/// Deletes the libraries of earlier builds, so this directory holds exactly one.
fn prune(root: &Path, keep: &Path) {
    for entry in std::fs::read_dir(root).into_iter().flatten().flatten() {
        let path = entry.path();
        if path != keep && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

fn text(path: &str) -> Option<&'static str> {
    std::str::from_utf8(asset(path)?).ok()
}

fn text_of(file: &File) -> &'static str {
    std::str::from_utf8(file.bytes).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icons_and_the_logo_are_embedded() {
        assert!(asset("icons/package.svg").is_some());
        assert!(asset("logo.png").is_some());
    }

    #[test]
    fn only_what_the_app_reads_is_embedded() {
        // `debris.ico` is a link-time resource and `scripts/README.md` documents the
        // repository; neither is ever opened at runtime.
        assert!(asset("debris.ico").is_none());
        assert!(!SCRIPTS.iter().any(|file| file.path.ends_with(".md")));
    }

    #[test]
    fn the_catalog_loads_and_recognises_a_system_package() {
        let catalog = system_catalog();
        assert!(catalog.len() > 30, "expected a real catalog, got {}", catalog.len());
        assert!(catalog.contains("Microsoft.WindowsStore_2024_x64__abc"));
    }

    #[test]
    fn the_shipped_preset_loads() {
        let presets = presets();
        assert!(!presets.is_empty());
        assert!(presets.iter().any(|preset| preset.name.contains("fresh start")));
    }

    #[test]
    fn the_script_library_unpacks_with_its_shape_and_loads() {
        let scripts = scripts().expect("library should unpack and load");
        assert!(scripts.len() >= 5);
        // The shared module is imported through `$PSScriptRoot`, so the tree matters.
        assert!(script_dir().join("lib/Debris.Common.psm1").is_file());
    }
}

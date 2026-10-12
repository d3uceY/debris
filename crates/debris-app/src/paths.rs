//! Where the shipped data lives.
//!
//! Two layouts have to work: a source checkout, where `assets/` and `scripts/` sit
//! beside `crates/`, and an installed layout, where they sit beside the executable.
//! The checkout wins, so `cargo run` needs no copy step.
//!
//! The checkout path is baked in at compile time. That is fine while this is a
//! repository-first app; a packaged build resolves everything beside the binary.

use debris_core::script::library;
use debris_core::{Preset, Result, ScriptMeta, SystemCatalog};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Layout {
    assets: PathBuf,
    scripts: PathBuf,
}

impl Layout {
    pub fn discover() -> Self {
        let checkout_assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let checkout_scripts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts");
        if checkout_assets.is_dir() {
            return Self {
                assets: checkout_assets,
                scripts: checkout_scripts,
            };
        }

        let beside_the_binary = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            assets: beside_the_binary.join("assets"),
            scripts: beside_the_binary.join("scripts"),
        }
    }

    pub fn scripts_dir(&self) -> &Path {
        &self.scripts
    }

    /// The assets root is what GPUI's `AssetSource` resolves `icons/...` against.
    pub fn icon_root(&self) -> PathBuf {
        self.assets.clone()
    }

    /// A missing or unreadable catalog is not fatal: it only means nothing is
    /// classified as system-critical, and the plan stops warning. That is a
    /// degradation worth continuing through, but worth saying out loud.
    pub fn system_catalog(&self) -> SystemCatalog {
        let path = self.assets.join("data/system-apps.txt");
        match SystemCatalog::from_file(&path) {
            Ok(catalog) => catalog,
            Err(error) => {
                eprintln!("debris: {error}");
                eprintln!("debris: continuing with an empty system catalog");
                SystemCatalog::default()
            }
        }
    }

    pub fn presets(&self) -> Vec<Preset> {
        match debris_core::preset::library(self.assets.join("data/presets")) {
            Ok(presets) => presets,
            Err(error) => {
                eprintln!("debris: {error}");
                Vec::new()
            }
        }
    }

    pub fn scripts(&self) -> Result<Vec<ScriptMeta>> {
        library(self.scripts_dir())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checkout_layout_is_found_and_its_data_files_exist() {
        let layout = Layout::discover();
        assert!(layout.icon_root().is_dir(), "assets directory should resolve");
        assert!(layout.scripts_dir().is_dir(), "scripts directory should resolve");
        assert!(layout.icon_root().join("icons/package.svg").is_file());
    }

    #[test]
    fn the_catalog_loads_and_recognises_a_system_package() {
        let catalog = Layout::discover().system_catalog();
        assert!(catalog.len() > 30, "expected a real catalog, got {}", catalog.len());
        assert!(catalog.contains("Microsoft.WindowsStore_2024_x64__abc"));
    }

    #[test]
    fn the_shipped_preset_loads() {
        let presets = Layout::discover().presets();
        assert!(!presets.is_empty());
        assert!(presets.iter().any(|preset| preset.name.contains("fresh start")));
    }

    #[test]
    fn the_script_library_loads() {
        let scripts = Layout::discover().scripts().expect("library should load");
        assert!(scripts.len() >= 5);
    }
}

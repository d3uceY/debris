//! The bloatware catalogue: what a package *is*, in one sentence.
//!
//! The inventory can say a package is installed. It cannot say whether the user
//! wants it, and `Microsoft.BingNews` does not answer that on its own. This is the
//! table that does, ported from the reference tool's database.
//!
//! It is advisory only. Nothing here gates a removal; it decides what the inspector
//! says and which packages the recommended set covers.

use crate::error::{DebrisError, Result};
use serde::{Deserialize, Serialize};

/// One known package.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BloatApp {
    /// Display name, e.g. `Bing News`.
    pub name: String,
    #[serde(default)]
    pub category: String,
    /// Matched against the package name, case-insensitively.
    pub package: String,
    #[serde(default)]
    pub description: String,
    /// Part of the set the catalogue recommends removing.
    #[serde(default)]
    pub default_selected: bool,
}

/// The catalogue, with the lookup the inventory asks for on every frame.
#[derive(Clone, Debug, Default)]
pub struct BloatCatalog {
    apps: Vec<BloatApp>,
}

impl BloatCatalog {
    pub fn parse(text: &str) -> Result<Self> {
        let apps: Vec<BloatApp> = serde_json::from_str(crate::json::strip_bom(text)).map_err(
            |error| DebrisError::Catalog {
                detail: error.to_string(),
            },
        )?;
        Ok(Self { apps })
    }

    pub fn len(&self) -> usize {
        self.apps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }

    pub fn apps(&self) -> &[BloatApp] {
        &self.apps
    }

    /// The entry for a package, if the catalogue knows it.
    ///
    /// Matching is *prefix* rather than equality, for the same reason the system
    /// catalogue uses prefixes: a package name is versioned and published in several
    /// flavours (`Microsoft.SkypeApp` against `Microsoft.SkypeApp_15.x`), and the
    /// inventory reports one while the data may carry the other. The match runs one
    /// way only — the reported name starting with the catalogued one. Matching the
    /// other way too would make a short query claim every longer entry.
    pub fn find(&self, package_name: &str) -> Option<&BloatApp> {
        let needle = package_name.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return None;
        }
        self.apps
            .iter()
            .find(|app| needle.starts_with(&app.package.to_ascii_lowercase()))
    }

    pub fn describe(&self, package_name: &str) -> Option<&str> {
        self.find(package_name)
            .map(|app| app.description.as_str())
            .filter(|text| !text.is_empty())
    }

    pub fn category_of(&self, package_name: &str) -> Option<&str> {
        self.find(package_name)
            .map(|app| app.category.as_str())
            .filter(|text| !text.is_empty())
    }

    /// The package names the catalogue recommends removing.
    pub fn recommended(&self) -> Vec<String> {
        self.apps
            .iter()
            .filter(|app| app.default_selected)
            .map(|app| app.package.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG: &str = r#"[
      { "name": "Bing News", "category": "Microsoft", "package": "Microsoft.BingNews",
        "description": "Bing-powered news feed.", "default_selected": true },
      { "name": "Skype", "category": "Office", "package": "Microsoft.SkypeApp",
        "description": "Consumer Skype app." }
    ]"#;

    #[test]
    fn finds_an_entry_by_package_name() {
        let catalog = BloatCatalog::parse(CATALOG).unwrap();
        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog.describe("Microsoft.BingNews"), Some("Bing-powered news feed."));
        assert_eq!(catalog.category_of("Microsoft.SkypeApp"), Some("Office"));
    }

    #[test]
    fn matching_ignores_case_and_a_version_suffix() {
        let catalog = BloatCatalog::parse(CATALOG).unwrap();
        assert!(catalog.find("microsoft.bingnews").is_some());
        assert!(catalog.find("Microsoft.BingNews_4.12.0.0_x64__abc").is_some());
    }

    #[test]
    fn an_unknown_package_is_not_described() {
        let catalog = BloatCatalog::parse(CATALOG).unwrap();
        assert_eq!(catalog.describe("Microsoft.WindowsCalculator"), None);
        assert_eq!(catalog.describe(""), None);
    }

    #[test]
    fn only_the_recommended_entries_are_listed() {
        let catalog = BloatCatalog::parse(CATALOG).unwrap();
        assert_eq!(catalog.recommended(), vec!["Microsoft.BingNews".to_string()]);
    }

    #[test]
    fn a_broken_catalogue_is_an_error() {
        assert!(BloatCatalog::parse("[{").is_err());
    }
}

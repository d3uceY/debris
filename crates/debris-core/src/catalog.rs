use crate::error::{DebrisError, Result};
use std::path::Path;

/// The packages Windows itself needs.
///
/// Anything matching this list is a system component: hidden from the inventory by
/// default, flagged in a removal plan, and never silently staged. This is the
/// difference between a cleanup tool and a machine that will not boot.
///
/// Matching is a **case-insensitive prefix** against the package name, because a
/// catalog entry names a package family: `Microsoft.WindowsStore` has to match
/// `Microsoft.WindowsStore_2024.1203.1.0_x64__8wekyb3d8bbwe`.
#[derive(Clone, Debug, Default)]
pub struct SystemCatalog {
    entries: Vec<String>,
}

impl SystemCatalog {
    /// One entry per line. Blank lines and `#` comments are ignored.
    pub fn parse(text: &str) -> Self {
        let entries = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.to_ascii_lowercase())
            .collect();
        Self { entries }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| DebrisError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self::parse(&text))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn contains(&self, package_name: &str) -> bool {
        let needle = package_name.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return false;
        }
        self.entries
            .iter()
            .any(|entry| needle.starts_with(entry.as_str()))
    }

    /// Applies the classification to a package. Kept here so there is exactly one
    /// place in the codebase that decides what "system" means.
    pub fn classify(&self, package: &mut crate::model::AppxPackage) {
        package.is_system = self.contains(&package.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AppxPackage;

    const SAMPLE: &str = "\
# system packages
Microsoft.WindowsStore

Microsoft.WindowsCalculator
InputApp
";

    #[test]
    fn ignores_comments_and_blank_lines() {
        let catalog = SystemCatalog::parse(SAMPLE);
        assert_eq!(catalog.len(), 3);
    }

    #[test]
    fn matches_exact_names() {
        let catalog = SystemCatalog::parse(SAMPLE);
        assert!(catalog.contains("Microsoft.WindowsCalculator"));
    }

    #[test]
    fn matches_package_families_by_prefix() {
        let catalog = SystemCatalog::parse(SAMPLE);
        assert!(catalog.contains("Microsoft.WindowsStore_2024.1203.1.0_x64__8wekyb3d8bbwe"));
        assert!(catalog.contains("InputApp_cw5n1h2txyewy"));
    }

    #[test]
    fn matching_is_case_insensitive() {
        let catalog = SystemCatalog::parse(SAMPLE);
        assert!(catalog.contains("microsoft.windowsstore"));
    }

    #[test]
    fn does_not_match_unrelated_packages() {
        let catalog = SystemCatalog::parse(SAMPLE);
        assert!(!catalog.contains("Microsoft.BingNews"));
        // A prefix must run the right way round: the catalog entry is the prefix.
        assert!(!catalog.contains("Microsoft"));
    }

    #[test]
    fn empty_catalog_matches_nothing() {
        let catalog = SystemCatalog::default();
        assert!(catalog.is_empty());
        assert!(!catalog.contains("Microsoft.WindowsStore"));
    }

    #[test]
    fn empty_needle_is_not_a_match() {
        let catalog = SystemCatalog::parse(SAMPLE);
        assert!(!catalog.contains(""));
        assert!(!catalog.contains("   "));
    }

    #[test]
    fn classify_marks_system_packages() {
        let catalog = SystemCatalog::parse(SAMPLE);
        let mut package = AppxPackage::new(
            "Microsoft.WindowsStore_2024_x64__8wekyb3d8bbwe",
            "Microsoft.WindowsStore_2024_x64__8wekyb3d8bbwe",
        );
        catalog.classify(&mut package);
        assert!(package.is_system);
    }
}

//! What the app remembers between launches.
//!
//! Small on purpose. Every field here is one the user set deliberately, so a fresh
//! profile with no settings file behaves identically to one whose file was deleted:
//! the defaults are the behaviour.

use crate::json::strip_bom;
use serde::{Deserialize, Serialize};

/// The persisted settings document.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Write a `.reg` backup before a batch of fixers is applied.
    pub backup_before_apply: bool,
    /// The dark token set, if the user chose it.
    pub dark: bool,
    /// Catalogue URLs the user added. Each is fetched on demand, never polled.
    pub extension_sources: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // On by default: the backup is the only thing that makes a fixer batch
            // reversible, and a user who never opens settings should still get it.
            backup_before_apply: true,
            dark: false,
            extension_sources: Vec::new(),
        }
    }
}

impl Settings {
    /// Reads a settings file. A file that does not parse yields the defaults rather
    /// than an error: settings are a convenience, and refusing to start over a
    /// malformed one would be the tail wagging the dog.
    ///
    /// A byte-order mark is tolerated, because editors and PowerShell both add one and
    /// a BOM would otherwise turn a perfectly good file into an empty profile.
    pub fn parse(text: &str) -> Self {
        serde_json::from_str(strip_bom(text)).unwrap_or_default()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// Whether a URL is already a source, so adding one twice is a no-op.
    pub fn has_source(&self, url: &str) -> bool {
        let url = normalise_url(url);
        self.extension_sources
            .iter()
            .any(|existing| normalise_url(existing) == url)
    }

    /// Adds a source and reports whether it was new.
    pub fn add_source(&mut self, url: &str) -> bool {
        let trimmed = url.trim();
        if trimmed.is_empty() || self.has_source(trimmed) {
            return false;
        }
        self.extension_sources.push(trimmed.to_string());
        true
    }

    pub fn remove_source(&mut self, url: &str) -> bool {
        let before = self.extension_sources.len();
        self.extension_sources.retain(|existing| existing != url);
        self.extension_sources.len() != before
    }
}

/// Trailing slashes and case are not part of a catalogue's identity.
fn normalise_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_safe_ones() {
        let settings = Settings::default();
        assert!(settings.backup_before_apply);
        assert!(!settings.dark);
        assert!(settings.extension_sources.is_empty());
    }

    #[test]
    fn a_settings_file_saved_with_a_byte_order_mark_still_loads() {
        // PowerShell's `Set-Content -Encoding UTF8` writes one, and reading it as an
        // empty profile would silently forget every setting the user has.
        let text = "\u{feff}{\"dark\": true, \"extension_sources\": [\"https://a/b.json\"]}";
        let settings = Settings::parse(text);
        assert!(settings.dark);
        assert_eq!(settings.extension_sources, vec!["https://a/b.json".to_string()]);
    }

    #[test]
    fn a_malformed_file_falls_back_to_the_defaults() {
        assert_eq!(Settings::parse("{ not json"), Settings::default());
        assert_eq!(Settings::parse(""), Settings::default());
    }

    #[test]
    fn settings_survive_a_round_trip() {
        let mut settings = Settings::default();
        settings.add_source("https://example.com/catalog.json");
        settings.dark = true;
        let back = Settings::parse(&settings.to_json());
        assert_eq!(settings, back);
    }

    #[test]
    fn a_missing_field_takes_its_default() {
        let settings = Settings::parse(r#"{"dark": true}"#);
        assert!(settings.dark);
        assert!(settings.backup_before_apply, "unmentioned fields keep defaults");
    }

    #[test]
    fn adding_the_same_source_twice_is_a_no_op() {
        let mut settings = Settings::default();
        assert!(settings.add_source("https://example.com/catalog.json"));
        assert!(!settings.add_source("https://example.com/catalog.json/"));
        assert!(!settings.add_source("  https://EXAMPLE.com/catalog.json  "));
        assert_eq!(settings.extension_sources.len(), 1);
        assert!(settings.has_source("https://example.com/catalog.json"));
    }

    #[test]
    fn an_empty_source_is_rejected_and_removing_one_works() {
        let mut settings = Settings::default();
        assert!(!settings.add_source("   "));
        settings.add_source("https://a/b.json");
        assert!(settings.remove_source("https://a/b.json"));
        assert!(!settings.remove_source("https://a/b.json"));
    }
}

use crate::error::{DebrisError, Result};
use std::path::{Path, PathBuf};

/// A named set of package names — "Fresh Start", or one the user saved.
///
/// The format is deliberately the plainest thing that works: one package name per
/// line, `#` for comments, blank lines ignored. Bloatbox used an XML
/// `<Config><Bloatware><string>` document for this; a diffable text file is a
/// better citizen in a repository and needs no parser. `scripts/presets/`
/// carries the converter for importing the old format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preset {
    pub name: String,
    pub packages: Vec<String>,
}

impl Preset {
    pub fn parse(name: impl Into<String>, text: &str) -> Self {
        let mut packages: Vec<String> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // Trim a trailing inline comment only when it is clearly separated,
            // so a package name is never truncated by a stray '#'.
            let entry = line.split_whitespace().next().unwrap_or("").trim();
            if entry.is_empty() {
                continue;
            }
            if !packages.iter().any(|existing| existing == entry) {
                packages.push(entry.to_string());
            }
        }
        Self {
            name: name.into(),
            packages,
        }
    }

    /// The preset's name is its file stem, so the filename is the identity.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path).map_err(|source| DebrisError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let name = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().replace('-', " "))
            .unwrap_or_else(|| "preset".to_string());
        Ok(Self::parse(name, &text))
    }

    pub fn to_text(&self) -> String {
        let mut out = String::from("# debris preset — one package name per line\n");
        for package in &self.packages {
            out.push_str(package);
            out.push('\n');
        }
        out
    }

    pub fn len(&self) -> usize {
        self.packages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.packages.is_empty()
    }
}

/// Reads the `<string>` entries out of a Bloatbox `Freshstart.xml`-style preset.
///
/// The original tool serialised a `List<string>` through .NET's XML serialiser, so the
/// document is always the same two-element shape. A targeted extractor for that one
/// shape is more honest than pulling an XML parser in as a dependency, and it is
/// tested against the real document under `references/`.
///
/// Unclosed tags stop the scan rather than erroring: a truncated preset should import
/// what it can, and the caller reports how many names landed.
pub fn import_bloatbox_xml(text: &str) -> Vec<String> {
    const OPEN: &str = "<string>";
    const CLOSE: &str = "</string>";

    let mut packages: Vec<String> = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(OPEN) {
        let after_open = &rest[start + OPEN.len()..];
        let Some(end) = after_open.find(CLOSE) else {
            break;
        };
        let value = after_open[..end].trim();
        if !value.is_empty() && !packages.iter().any(|existing| existing == value) {
            packages.push(value.to_string());
        }
        rest = &after_open[end + CLOSE.len()..];
    }
    packages
}

/// Every `.txt` preset in a directory, sorted by name. A missing directory is an
/// empty list, not an error — shipping no presets is a valid state.
pub fn library(dir: impl AsRef<Path>) -> Result<Vec<Preset>> {
    let dir = dir.as_ref();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut presets = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|source| DebrisError::Io {
        path: PathBuf::from(dir),
        source,
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("txt") {
            presets.push(Preset::from_file(path)?);
        }
    }
    presets.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(presets)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_one_name_per_line() {
        let preset = Preset::parse("Fresh Start", "Microsoft.BingNews\nSpotifyAB.SpotifyMusic\n");
        assert_eq!(preset.len(), 2);
        assert_eq!(preset.packages[0], "Microsoft.BingNews");
    }

    #[test]
    fn ignores_comments_and_blank_lines() {
        let preset = Preset::parse(
            "x",
            "# a comment\n\nMicrosoft.BingNews\n   \n# another\nMicrosoft.ZuneMusic\n",
        );
        assert_eq!(preset.len(), 2);
    }

    #[test]
    fn deduplicates_preserving_order() {
        let preset = Preset::parse("x", "A\nB\nA\n");
        assert_eq!(preset.packages, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn tolerates_trailing_whitespace_and_crlf() {
        let preset = Preset::parse("x", "Microsoft.BingNews  \r\nMicrosoft.ZuneMusic\r\n");
        assert_eq!(preset.packages, vec!["Microsoft.BingNews", "Microsoft.ZuneMusic"]);
    }

    #[test]
    fn an_empty_document_is_an_empty_preset() {
        assert!(Preset::parse("x", "# nothing here\n").is_empty());
    }

    #[test]
    fn round_trips_through_text() {
        let preset = Preset::parse("x", "A\nB\n");
        let reparsed = Preset::parse("x", &preset.to_text());
        assert_eq!(preset.packages, reparsed.packages);
    }

    #[test]
    fn a_missing_directory_is_an_empty_library() {
        let presets = library("C:/definitely/not/here").unwrap();
        assert!(presets.is_empty());
    }

    #[test]
    fn reads_the_shipped_preset_directory() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/data/presets");
        let presets = library(dir).unwrap();
        assert!(!presets.is_empty(), "expected at least one shipped preset");
        assert!(presets.iter().any(|p| p.name.contains("fresh start")));
    }

    #[test]
    fn imports_a_bloatbox_xml_preset() {
        let xml = r#"<?xml version="1.0"?>
<Config><Bloatware>
<string>4DF9E0F8.Netflix</string>
<string>Microsoft.BingNews</string>
</Bloatware></Config>"#;
        assert_eq!(
            import_bloatbox_xml(xml),
            vec!["4DF9E0F8.Netflix", "Microsoft.BingNews"]
        );
    }

    #[test]
    fn importing_tolerates_whitespace_and_deduplicates() {
        let xml = "<string>\n  Microsoft.ZuneMusic\n</string><string>Microsoft.ZuneMusic</string>";
        assert_eq!(import_bloatbox_xml(xml), vec!["Microsoft.ZuneMusic"]);
    }

    #[test]
    fn an_unclosed_tag_keeps_what_came_before_it() {
        let xml = "<string>Microsoft.BingNews</string><string>dangling";
        assert_eq!(import_bloatbox_xml(xml), vec!["Microsoft.BingNews"]);
    }

    #[test]
    fn importing_a_document_without_entries_yields_nothing() {
        assert!(import_bloatbox_xml("<Config><Bloatware /></Config>").is_empty());
        assert!(import_bloatbox_xml("").is_empty());
    }

    #[test]
    fn imports_the_real_bloatbox_reference_preset() {
        // The reference copy is vendored, not built; skip if it is not checked out.
        let doc = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../references/bloatbox/src/Bloatbox/Resources/Freshstart.xml"
        );
        let Ok(text) = std::fs::read_to_string(doc) else {
            return;
        };
        let packages = import_bloatbox_xml(&text);
        assert!(packages.len() > 30, "got {} packages", packages.len());
        assert!(packages.iter().any(|name| name == "Microsoft.BingNews"));
    }
}

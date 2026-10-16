//! The extension catalogue: third-party PowerShell, listed from a JSON file.
//!
//! A catalogue is a list of scripts this app does not ship and does not audit. It
//! names them, says where to fetch each one, and stops there. Installing copies the
//! file into the app's own data directory; running it is a separate, confirmed act.
//!
//! The format is deliberately forgiving: the file is written by someone else, on
//! someone else's build schedule, and a catalogue that is one field away from the
//! shape this app expected should still load.

use crate::error::{DebrisError, Result};
use serde::{Deserialize, Serialize};

/// One installable script.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Extension {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Empty when the catalogue forgot it. The entry is then dropped by
    /// [`Extension::parse_catalog`] — but only that entry, not the whole file.
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub version: String,
    /// Shown before anything runs, and checked where the catalogue supplies one.
    #[serde(default, alias = "requires_admin", rename = "requiresAdmin")]
    pub requires_admin: bool,
    /// Optional. When present, a downloaded file that does not match is discarded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

/// There is one document shape: an array of entries, which is what this app defines
/// and what its documentation shows. Accepting a `{"extensions": [...]}` wrapper too,
/// on the guess that some catalogue somewhere uses one, would be a second format to
/// support and a second thing for every reader to know about.
#[derive(Deserialize)]
#[serde(transparent)]
struct Catalog(Vec<Extension>);

impl Extension {
    /// Parses a catalogue, keeping only entries this app can actually install.
    ///
    /// Dropping a bad entry rather than failing the file is the right trade: one
    /// author's typo in a shared catalogue should not hide everyone else's scripts.
    pub fn parse_catalog(text: &str) -> Result<Vec<Self>> {
        let Catalog(entries): Catalog =
            serde_json::from_str(crate::json::strip_bom(text)).map_err(|error| {
                DebrisError::Catalog {
                    detail: error.to_string(),
                }
            })?;
        let mut kept: Vec<Self> = entries
            .into_iter()
            .filter(|entry| !entry.name.trim().is_empty())
            .filter(|entry| Self::file_name(&entry.url).is_some())
            .collect();
        kept.sort_by_cached_key(|entry| entry.name.to_ascii_lowercase());
        Ok(kept)
    }

    /// The local filename for this entry's script.
    ///
    /// Taken from the tail of the URL, never from the catalogue's own text: a
    /// catalogue is third-party input, and a name it controls that reaches the
    /// filesystem is a path-traversal waiting to happen.
    pub fn file_name(url: &str) -> Option<String> {
        let without_query = url.split(['?', '#']).next().unwrap_or(url);
        let tail = without_query.rsplit('/').next()?;
        let decoded = percent_decode(tail);
        if !decoded.to_ascii_lowercase().ends_with(".ps1") {
            return None;
        }
        if decoded.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) || decoded.starts_with('.')
        {
            return None;
        }
        Some(decoded)
    }

    pub fn file_name_of(&self) -> Option<String> {
        Self::file_name(&self.url)
    }

    /// Whether this entry is safe to fetch at all.
    pub fn is_fetchable(&self) -> bool {
        is_fetchable(&self.url)
    }

    /// A `v1.0` chip, or nothing when the catalogue did not say.
    pub fn version_label(&self) -> Option<String> {
        let version = self.version.trim();
        (!version.is_empty()).then(|| format!("v{version}"))
    }
}

/// Whether a string is an address this app will fetch.
///
/// Only HTTP(S). A `file:` URL would turn "add a catalogue" into "read whatever path
/// I name", which is a different feature from the one on offer.
pub fn is_fetchable(url: &str) -> bool {
    let trimmed = url.trim().to_ascii_lowercase();
    trimmed.starts_with("https://") || trimmed.starts_with("http://")
}

/// The address that serves the file, given the address a browser showed for it.
///
/// On GitHub those are two different URLs, and the one people copy is the page:
/// `github.com/o/r/blob/main/catalog.json` serves the viewer's HTML — and answers a
/// non-browser client with a bare 503 — while `raw.githubusercontent.com/o/r/main/…`
/// serves the bytes. Rewriting the page URL is what makes the address bar a valid way
/// to add a catalogue. Anything that is not a GitHub page URL comes back untouched.
pub fn raw_url(url: &str) -> String {
    const PAGES: [&str; 4] = [
        "https://github.com/",
        "http://github.com/",
        "https://www.github.com/",
        "http://www.github.com/",
    ];

    let trimmed = url.trim();
    let Some(rest) = PAGES.iter().find_map(|page| trimmed.strip_prefix(page)) else {
        return trimmed.to_string();
    };
    // `<owner>/<repo>/<blob|raw>/<ref>/<path>`; the ref is handed through whole so a
    // branch with a slash in it is not mangled.
    let mut parts = rest.splitn(4, '/');
    let (Some(owner), Some(repo), Some(kind), Some(tail)) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return trimmed.to_string();
    };
    // A `#L12` from a line-numbered view is not part of the file's address.
    let tail = tail.split(['?', '#']).next().unwrap_or(tail);
    if owner.is_empty()
        || repo.is_empty()
        || !matches!(kind, "blob" | "raw")
        || !tail.contains('/')
    {
        return trimmed.to_string();
    }
    format!("https://raw.githubusercontent.com/{owner}/{repo}/{tail}")
}

/// Decodes `%20` and friends, because script URLs are usually full of them.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or("");
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                out.push(byte as char);
                index += 3;
                continue;
            }
        }
        out.push(bytes[index] as char);
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const BARE: &str = r#"[
      { "name": "Restart Explorer", "description": "Restart the shell.",
        "url": "https://example.com/plugins/Restart%20Explorer.ps1", "version": "1.0",
        "requiresAdmin": false },
      { "name": "Restore Point", "url": "https://example.com/plugins/make-point.ps1",
        "requires_admin": true }
    ]"#;

    #[test]
    fn parses_a_bare_array_and_both_spellings_of_the_admin_flag() {
        let entries = Extension::parse_catalog(BARE).unwrap();
        assert_eq!(entries.len(), 2);
        // Sorted by name, so "Restart Explorer" precedes "Restore Point".
        assert_eq!(entries[0].name, "Restart Explorer");
        assert!(!entries[0].requires_admin);
        assert!(entries[1].requires_admin);
    }

    #[test]
    fn entries_that_are_not_scripts_are_dropped_rather_than_failing_the_file() {
        let text = r#"[
          { "name": "Good", "url": "https://a/good.ps1" },
          { "name": "Not a script", "url": "https://a/readme.md" },
          { "name": "No url" },
          { "name": "", "url": "https://a/nameless.ps1" }
        ]"#;
        let entries = Extension::parse_catalog(text).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Good");
    }

    #[test]
    fn the_local_name_comes_from_the_url_and_is_percent_decoded() {
        assert_eq!(
            Extension::file_name("https://x/y/Restart%20Explorer.ps1"),
            Some("Restart Explorer.ps1".to_string())
        );
        assert_eq!(
            Extension::file_name("https://x/y/z.ps1?token=1#frag"),
            Some("z.ps1".to_string())
        );
    }

    #[test]
    fn a_catalogue_cannot_name_a_file_outside_its_own_folder() {
        // The name is built from the URL tail, so a traversal has to arrive there.
        assert_eq!(Extension::file_name("https://x/..%5Cevil.ps1"), None);
        assert_eq!(Extension::file_name("https://x/.hidden.ps1"), None);
        assert_eq!(Extension::file_name("https://x/a.ps1.ps1"), Some("a.ps1.ps1".into()));
    }

    #[test]
    fn a_github_page_url_becomes_the_address_that_serves_the_file() {
        assert_eq!(
            raw_url("https://github.com/d3uceY/debris/blob/main/examples/extensions/catalog.json"),
            "https://raw.githubusercontent.com/d3uceY/debris/main/examples/extensions/catalog.json"
        );
        // The `raw` spelling, a query string, and a line-numbered link all land on the
        // same address; the ref is passed through whole, slashes and all.
        assert_eq!(
            raw_url("https://github.com/o/r/raw/v2/x.json?token=1"),
            "https://raw.githubusercontent.com/o/r/v2/x.json"
        );
        assert_eq!(
            raw_url("https://github.com/o/r/blob/feat/x/y.json#L12"),
            "https://raw.githubusercontent.com/o/r/feat/x/y.json"
        );
    }

    #[test]
    fn a_url_that_is_not_a_github_file_page_is_left_alone() {
        for url in [
            // A directory listing is not a file.
            "https://github.com/o/r/tree/main",
            // No path after the ref.
            "https://github.com/o/r/blob/main",
            // Not GitHub, and already raw.
            "https://raw.githubusercontent.com/o/r/main/x.json",
            "https://example.com/x.json",
            "file:///C:/x.json",
        ] {
            assert_eq!(raw_url(url), url);
        }
        // Trailing space is the clipboard's, not the address's.
        assert_eq!(raw_url("  https://example.com/x.json "), "https://example.com/x.json");
    }

    #[test]
    fn only_http_urls_are_fetchable() {
        let mut entry = Extension {
            name: "x".into(),
            description: String::new(),
            url: "file:///C:/evil.ps1".into(),
            version: String::new(),
            requires_admin: false,
            sha256: None,
        };
        assert!(!entry.is_fetchable());
        entry.url = "https://example.com/x.ps1".into();
        assert!(entry.is_fetchable());
    }

    #[test]
    fn a_version_is_only_shown_when_one_was_given() {
        let mut entry: Extension = Extension::parse_catalog(BARE).unwrap().remove(0);
        assert_eq!(entry.version_label(), Some("v1.0".to_string()));
        entry.version = String::new();
        assert_eq!(entry.version_label(), None);
    }

    #[test]
    fn a_broken_catalogue_is_an_error() {
        assert!(Extension::parse_catalog("{ oops").is_err());
    }
}

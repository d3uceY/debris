//! Extensions: fetching a catalogue, and fetching and checking one script from it.
//!
//! Thin by design. [`super::fetch`] owns the network call through the runner seam,
//! and the app owns the filesystem: where a script is written, whether it was
//! recorded as installed, and how a catalogue body is cached all depend on the
//! profile layout, which this crate deliberately does not know about.
//!
//! What lives here is the part with a rule in it — a download is only accepted when
//! the checksum its catalogue published matches what arrived.

use crate::fetch;
use crate::runner::CommandRunner;
use debris_core::{Extension, Result};
use std::path::Path;

/// Fetches a catalogue and parses it, returning the body alongside the entries so a
/// caller can cache exactly what was parsed rather than refetching it.
pub fn load(runner: &dyn CommandRunner, url: &str) -> Result<(String, Vec<Extension>)> {
    let body = fetch::fetch(runner, url)?;
    let extensions = Extension::parse_catalog(&body)?;
    Ok((body, extensions))
}

/// Checks a downloaded script against the checksum its catalogue published.
///
/// A catalogue without a checksum is accepted, because most in the wild have none and
/// refusing them would make the feature useless. An entry *with* one is not: the
/// author went to the trouble of publishing it, and ignoring it would be worse than
/// not supporting it at all.
pub fn verify(
    runner: &dyn CommandRunner,
    path: &Path,
    expected: Option<&str>,
) -> std::result::Result<(), String> {
    let Some(expected) = expected.filter(|hash| !hash.trim().is_empty()) else {
        return Ok(());
    };
    let actual = fetch::sha256(runner, path).map_err(|error| error.to_string())?;
    if fetch::checksum_matches(&actual, expected) {
        return Ok(());
    }
    Err(format!(
        "the downloaded copy does not match the published checksum (published {expected}, this copy is {actual})"
    ))
}

/// The version label to record for an installed extension. A catalogue that omitted
/// one is recorded as empty rather than as a guess.
pub fn recorded_version(extension: &Extension) -> String {
    extension.version.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{RunOutput, ScriptedRunner};

    const CATALOGUE: &str = r#"[
      { "name": "Restart Explorer", "url": "https://example.com/Restart%20Explorer.ps1", "version": "1.2" }
    ]"#;

    #[test]
    fn a_catalogue_is_returned_with_a_parsed_list() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, CATALOGUE)]);
        let (body, entries) = load(&runner, "https://example.com/catalog.json").unwrap();
        assert_eq!(body, CATALOGUE, "the body comes back so it can be cached");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].version_label(), Some("v1.2".to_string()));
    }

    #[test]
    fn a_catalogue_that_is_not_json_is_an_error() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "<html>nope</html>")]);
        assert!(load(&runner, "https://example.com/page.html").is_err());
    }

    #[test]
    fn an_extension_with_no_published_checksum_verifies() {
        let runner = ScriptedRunner::new(vec![]);
        assert!(verify(&runner, Path::new("C:/x.ps1"), None).is_ok());
        assert!(verify(&runner, Path::new("C:/x.ps1"), Some("  ")).is_ok());
        assert!(
            runner.log().is_empty(),
            "there is nothing to hash and no process should start"
        );
    }

    #[test]
    fn a_matching_checksum_verifies() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "AB12CD\r\n")]);
        assert!(verify(&runner, Path::new("C:/x.ps1"), Some("ab:12:cd")).is_ok());
    }

    #[test]
    fn a_mismatched_checksum_names_both_copies() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "AB12CD\r\n")]);
        let error = verify(&runner, Path::new("C:/x.ps1"), Some("FFFF")).unwrap_err();
        assert!(error.contains("FFFF"), "{error}");
        assert!(error.contains("AB12CD"), "{error}");
    }

    #[test]
    fn a_checksum_that_cannot_be_read_is_an_error_not_a_pass() {
        let runner = ScriptedRunner::new(vec![RunOutput {
            status: 1,
            stdout: String::new(),
            stderr: "Cannot find path".to_string(),
        }]);
        assert!(verify(&runner, Path::new("C:/x.ps1"), Some("AB12")).is_err());
    }

    #[test]
    fn an_omitted_version_is_recorded_as_empty() {
        let mut extension = Extension {
            name: "x".into(),
            description: String::new(),
            url: "https://example.com/x.ps1".into(),
            version: "   ".into(),
            requires_admin: false,
            sha256: None,
        };
        assert_eq!(recorded_version(&extension), "");
        extension.version = "2.0".into();
        assert_eq!(recorded_version(&extension), "2.0");
    }
}

use crate::error::{DebrisError, Result};
use std::path::{Path, PathBuf};

/// How much a script can change, as declared by the script itself.
///
/// This is the load-bearing type of the whole safety model: the library trusts a
/// script's own declaration, and a script that declares `destructive` without
/// supporting a dry run is refused rather than merely warned about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    /// Reads state and reports. Changes nothing.
    ReadOnly,
    /// Changes state, and the change can be undone by the user.
    Reversible,
    /// Removes or disables something that may not come back.
    Destructive,
}

impl Risk {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "read-only" | "readonly" | "read only" => Some(Self::ReadOnly),
            "reversible" => Some(Self::Reversible),
            "destructive" => Some(Self::Destructive),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "READ-ONLY",
            Self::Reversible => "REVERSIBLE",
            Self::Destructive => "DESTRUCTIVE",
        }
    }

    /// One line, in the product's voice, saying what the label means. Shown to the
    /// user next to the tag so the tag never has to be learned.
    pub fn detail(self) -> &'static str {
        match self {
            Self::ReadOnly => "Reports only. Nothing on this machine changes.",
            Self::Reversible => "Changes settings you can change back.",
            Self::Destructive => "Removes or disables things that may not come back.",
        }
    }

    pub fn is_destructive(self) -> bool {
        matches!(self, Self::Destructive)
    }
}

/// Which shelf a script sits on. Matches the `scripts/` subdirectories.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    Apps,
    Privacy,
    Shell,
    Maintenance,
}

impl Category {
    pub const ALL: [Category; 4] = [
        Category::Apps,
        Category::Privacy,
        Category::Shell,
        Category::Maintenance,
    ];

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "apps" | "app" => Some(Self::Apps),
            "privacy" => Some(Self::Privacy),
            "shell" => Some(Self::Shell),
            "maintenance" => Some(Self::Maintenance),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Apps => "Apps",
            Self::Privacy => "Privacy",
            Self::Shell => "Shell",
            Self::Maintenance => "Maintenance",
        }
    }

    pub fn directory(self) -> &'static str {
        match self {
            Self::Apps => "apps",
            Self::Privacy => "privacy",
            Self::Shell => "shell",
            Self::Maintenance => "maintenance",
        }
    }
}

/// A script in the library, described entirely by the `###` header at the top of
/// the file. No sidecar metadata, no manifest to drift out of sync: the script and
/// its description are the same file, and a script without a valid header does not
/// load at all.
///
/// ```text
/// ### title: Remove OneDrive
/// ### category: apps
/// ### risk: reversible
/// ### requires-admin: no
/// ### dry-run: yes
/// ###
/// ### Removes the OneDrive client and disables its sync policy.
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptMeta {
    /// File stem — the stable identity a preset or a log line refers to.
    pub id: String,
    pub path: PathBuf,
    pub title: String,
    pub category: Category,
    pub risk: Risk,
    pub requires_admin: bool,
    pub supports_dry_run: bool,
    pub author: Option<String>,
    /// The prose lines from the header, in order.
    pub summary: Vec<String>,
}

const MARKER: &str = "###";

impl ScriptMeta {
    pub fn parse(path: &Path, text: &str) -> Result<Self> {
        let id = path
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut title: Option<String> = None;
        let mut category: Option<Category> = None;
        let mut risk: Option<Risk> = None;
        let mut dry_run: Option<bool> = None;
        let mut requires_admin = false;
        let mut author: Option<String> = None;
        let mut summary: Vec<String> = Vec::new();
        let mut problems: Vec<String> = Vec::new();

        let mut opened = false;
        let mut in_header = true;
        for line in text.lines() {
            let trimmed = line.trim();

            if !in_header {
                break;
            }
            if trimmed.is_empty() {
                // Leading blanks are tolerated; the first blank *after* the header
                // closes it, so a `###` comment deeper in the file is just a comment.
                if opened {
                    in_header = false;
                }
                continue;
            }
            let Some(body) = trimmed.strip_prefix(MARKER) else {
                break;
            };
            opened = true;

            let body = body.trim();
            if body.is_empty() {
                continue;
            }

            match body.split_once(':') {
                Some((key, value)) if directive(key).is_some() => {
                    let value = value.trim();
                    match directive(key).unwrap_or_default() {
                        "title" => title = Some(value.to_string()),
                        "category" => match Category::parse(value) {
                            Some(parsed) => category = Some(parsed),
                            None => problems.push(format!("unknown category `{value}`")),
                        },
                        "risk" => match Risk::parse(value) {
                            Some(parsed) => risk = Some(parsed),
                            None => problems.push(format!("unknown risk `{value}`")),
                        },
                        "requires-admin" => match parse_flag(value) {
                            Some(flag) => requires_admin = flag,
                            None => problems
                                .push(format!("`requires-admin: {value}` is not yes or no")),
                        },
                        "dry-run" => match parse_flag(value) {
                            Some(flag) => dry_run = Some(flag),
                            None => problems.push(format!("`dry-run: {value}` is not yes or no")),
                        },
                        "author" => author = Some(value.to_string()),
                        _ => summary.push(body.to_string()),
                    }
                }
                // Not a directive: prose. `Author of this script: ...` lands here,
                // which is exactly what we want.
                _ => summary.push(body.to_string()),
            }
        }

        if title.is_none() {
            problems.push("missing `title:`".to_string());
        }
        if category.is_none() {
            problems.push("missing or invalid `category:`".to_string());
        }
        if risk.is_none() {
            problems.push("missing or invalid `risk:`".to_string());
        }
        if dry_run.is_none() {
            problems.push("missing `dry-run:`".to_string());
        }
        if !problems.is_empty() {
            return Err(DebrisError::ScriptHeader {
                path: path.to_path_buf(),
                detail: problems.join("; "),
            });
        }

        Ok(Self {
            id,
            path: path.to_path_buf(),
            title: title.unwrap_or_default(),
            category: category.unwrap_or(Category::Maintenance),
            risk: risk.unwrap_or(Risk::ReadOnly),
            requires_admin,
            supports_dry_run: dry_run.unwrap_or(false),
            author,
            summary,
        })
    }

    /// A destructive script that cannot be dry-run is not offered. The rule lives
    /// here so the UI, the CLI, and the tests all enforce the same one.
    pub fn is_runnable(&self) -> bool {
        !(self.risk.is_destructive() && !self.supports_dry_run)
    }

    /// Why a script is not runnable, for the interface to say out loud.
    pub fn blocked_reason(&self) -> Option<&'static str> {
        if self.is_runnable() {
            None
        } else {
            Some("Declares destructive risk without supporting -DryRun.")
        }
    }

    pub fn summary_text(&self) -> String {
        self.summary.join(" ")
    }
}

fn directive(key: &str) -> Option<&'static str> {
    match key.trim().to_ascii_lowercase().as_str() {
        "title" => Some("title"),
        "category" => Some("category"),
        "risk" => Some("risk"),
        "requires-admin" | "admin" => Some("requires-admin"),
        "dry-run" | "dryrun" => Some("dry-run"),
        "author" => Some("author"),
        _ => None,
    }
}

fn parse_flag(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "yes" | "true" | "always" => Some(true),
        "no" | "false" | "never" => Some(false),
        _ => None,
    }
}

/// Every script under `root`, recursive, sorted by category then title.
///
/// `lib/` is skipped: shared PowerShell modules are not user-facing scripts. A
/// script with an invalid header fails the whole load rather than disappearing
/// quietly, because a script nobody can describe is a script nobody should run.
pub fn library(root: impl AsRef<Path>) -> Result<Vec<ScriptMeta>> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Err(DebrisError::LibraryMissing {
            path: root.to_path_buf(),
        });
    }
    let mut scripts = Vec::new();
    walk(root, &mut scripts)?;
    scripts.sort_by(|a, b| a.category.cmp(&b.category).then(a.title.cmp(&b.title)));
    Ok(scripts)
}

fn walk(dir: &Path, out: &mut Vec<ScriptMeta>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|source| DebrisError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("lib") {
                continue;
            }
            walk(&path, out)?;
            continue;
        }
        let is_ps1 = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ps1"));
        if !is_ps1 {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|source| DebrisError::Io {
            path: path.clone(),
            source,
        })?;
        out.push(ScriptMeta::parse(&path, &text)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(name: &str) -> PathBuf {
        PathBuf::from("scripts").join(name)
    }

    const GOOD: &str = "\
### title: Remove OneDrive
### category: apps
### risk: reversible
### requires-admin: yes
### dry-run: yes
###
### Removes the OneDrive client.
### Also clears its scheduled task.

Import-Module x
";

    #[test]
    fn parses_a_complete_header() {
        let meta = ScriptMeta::parse(&path("remove-onedrive.ps1"), GOOD).unwrap();
        assert_eq!(meta.title, "Remove OneDrive");
        assert_eq!(meta.category, Category::Apps);
        assert_eq!(meta.risk, Risk::Reversible);
        assert!(meta.requires_admin);
        assert!(meta.supports_dry_run);
        assert_eq!(meta.id, "remove-onedrive");
        assert_eq!(meta.summary.len(), 2);
        assert!(meta.is_runnable());
    }

    #[test]
    fn prose_with_a_colon_stays_prose() {
        let text = "\
### title: T
### category: apps
### risk: read-only
### dry-run: no
### Author of this script: https://example.com/x
";
        let meta = ScriptMeta::parse(&path("a.ps1"), text).unwrap();
        assert_eq!(meta.summary, vec!["Author of this script: https://example.com/x"]);
        assert_eq!(meta.author, None);
    }

    #[test]
    fn the_author_directive_is_captured() {
        let text = "\
### title: T
### category: apps
### risk: read-only
### dry-run: no
### author: W4RH4WK
";
        let meta = ScriptMeta::parse(&path("a.ps1"), text).unwrap();
        assert_eq!(meta.author.as_deref(), Some("W4RH4WK"));
        assert!(meta.summary.is_empty());
    }

    #[test]
    fn requires_admin_defaults_to_false() {
        let text = "### title: T\n### category: shell\n### risk: read-only\n### dry-run: yes\n";
        let meta = ScriptMeta::parse(&path("a.ps1"), text).unwrap();
        assert!(!meta.requires_admin);
    }

    #[test]
    fn a_comment_below_the_header_is_not_absorbed() {
        let text = "\
### title: T
### category: apps
### risk: read-only
### dry-run: yes

# just a comment
### not a header line

Write-Output hi
";
        let meta = ScriptMeta::parse(&path("a.ps1"), text).unwrap();
        assert!(meta.summary.is_empty());
    }

    #[test]
    fn a_script_without_a_header_is_rejected() {
        let err = ScriptMeta::parse(&path("bare.ps1"), "Write-Output 'no header'\n").unwrap_err();
        match err {
            DebrisError::ScriptHeader { detail, .. } => {
                assert!(detail.contains("title"));
                assert!(detail.contains("category"));
                assert!(detail.contains("risk"));
                assert!(detail.contains("dry-run"));
            }
            other => panic!("expected a header error, got {other:?}"),
        }
    }

    #[test]
    fn an_unknown_risk_is_rejected_by_name() {
        let text = "### title: T\n### category: apps\n### risk: spicy\n### dry-run: yes\n";
        let err = ScriptMeta::parse(&path("a.ps1"), text).unwrap_err();
        let DebrisError::ScriptHeader { detail, .. } = err else {
            panic!("expected a header error")
        };
        assert!(detail.contains("unknown risk `spicy`"));
    }

    #[test]
    fn an_unknown_category_is_rejected_by_name() {
        let text = "### title: T\n### category: vibes\n### risk: read-only\n### dry-run: yes\n";
        let err = ScriptMeta::parse(&path("a.ps1"), text).unwrap_err();
        let DebrisError::ScriptHeader { detail, .. } = err else {
            panic!("expected a header error")
        };
        assert!(detail.contains("unknown category `vibes`"));
    }

    #[test]
    fn a_non_boolean_flag_is_rejected() {
        let text = "### title: T\n### category: apps\n### risk: read-only\n### dry-run: maybe\n";
        let err = ScriptMeta::parse(&path("a.ps1"), text).unwrap_err();
        let DebrisError::ScriptHeader { detail, .. } = err else {
            panic!("expected a header error")
        };
        assert!(detail.contains("not yes or no"));
    }

    #[test]
    fn destructive_without_dry_run_is_not_runnable() {
        let text = "### title: T\n### category: apps\n### risk: destructive\n### dry-run: no\n";
        let meta = ScriptMeta::parse(&path("a.ps1"), text).unwrap();
        assert!(!meta.is_runnable());
        assert!(meta.blocked_reason().is_some());
    }

    #[test]
    fn destructive_with_dry_run_is_runnable() {
        let text = "### title: T\n### category: apps\n### risk: destructive\n### dry-run: yes\n";
        let meta = ScriptMeta::parse(&path("a.ps1"), text).unwrap();
        assert!(meta.is_runnable());
        assert!(meta.blocked_reason().is_none());
    }

    #[test]
    fn readonly_without_dry_run_is_still_runnable() {
        let text = "### title: T\n### category: apps\n### risk: read-only\n### dry-run: no\n";
        assert!(ScriptMeta::parse(&path("a.ps1"), text).unwrap().is_runnable());
    }

    #[test]
    fn risk_and_category_round_trip_through_their_labels() {
        for category in Category::ALL {
            assert_eq!(Category::parse(category.directory()), Some(category));
        }
        for risk in ["read-only", "reversible", "destructive"] {
            assert_eq!(Risk::parse(risk).map(Risk::parse_label_back), Some(risk));
        }
    }

    #[test]
    fn a_missing_library_directory_is_an_error_not_an_empty_list() {
        assert!(matches!(
            library("C:/definitely/not/here"),
            Err(DebrisError::LibraryMissing { .. })
        ));
    }

    #[test]
    fn loads_the_real_script_library_and_every_script_is_runnable() {
        let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../scripts");
        let scripts = library(root).unwrap();
        assert!(scripts.len() >= 5, "expected the shipped library to be populated");
        for script in &scripts {
            assert!(
                script.is_runnable(),
                "{} is not runnable: {:?}",
                script.id,
                script.blocked_reason()
            );
            assert!(!script.summary.is_empty(), "{} has no summary", script.id);
        }
    }

    impl Risk {
        fn parse_label_back(self) -> &'static str {
            match self {
                Self::ReadOnly => "read-only",
                Self::Reversible => "reversible",
                Self::Destructive => "destructive",
            }
        }
    }
}

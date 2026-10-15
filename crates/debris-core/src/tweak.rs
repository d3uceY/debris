//! The fixer model: one registry rule, and whether the machine already satisfies it.
//!
//! A tweak is a small declarative rule, not a script. It names the registry values
//! it cares about, the value it wants there, and the value Windows ships with. That
//! third field is what makes the whole surface honest: without it a tweak could be
//! applied but never taken back, and "make it like new again" would be a promise the
//! app could not keep.
//!
//! Everything here is pure. Reading the registry happens in `debris-backend`, and
//! [`Tweak::scan`] takes the reading as a closure so the decision — applied or not —
//! is a value in and a value out.

use crate::error::{DebrisError, Result};
use serde::{Deserialize, Serialize};

/// Where a key lives. Normalised on parse, so `HKLM` and `HKEY_LOCAL_MACHINE` are
/// the same hive and a file may spell it either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Hive {
    CurrentUser,
    LocalMachine,
    ClassesRoot,
    Users,
}

impl Hive {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_uppercase().as_str() {
            "HKCU" | "HKEY_CURRENT_USER" => Some(Self::CurrentUser),
            "HKLM" | "HKEY_LOCAL_MACHINE" => Some(Self::LocalMachine),
            "HKCR" | "HKEY_CLASSES_ROOT" => Some(Self::ClassesRoot),
            "HKU" | "HKEY_USERS" => Some(Self::Users),
            _ => None,
        }
    }

    /// The short form, used in the UI and in the machinery below.
    pub fn short(self) -> &'static str {
        match self {
            Self::CurrentUser => "HKCU",
            Self::LocalMachine => "HKLM",
            Self::ClassesRoot => "HKCR",
            Self::Users => "HKU",
        }
    }

    /// The full form, which is what a `.reg` file insists on.
    pub fn long(self) -> &'static str {
        match self {
            Self::CurrentUser => "HKEY_CURRENT_USER",
            Self::LocalMachine => "HKEY_LOCAL_MACHINE",
            Self::ClassesRoot => "HKEY_CLASSES_ROOT",
            Self::Users => "HKEY_USERS",
        }
    }

    /// Writing anywhere but the current user needs an elevated process.
    pub fn needs_admin(self) -> bool {
        !matches!(self, Self::CurrentUser)
    }
}

impl TryFrom<String> for Hive {
    type Error = String;

    fn try_from(raw: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(&raw).ok_or_else(|| format!("unknown hive `{raw}`"))
    }
}

impl From<Hive> for String {
    fn from(hive: Hive) -> Self {
        hive.short().to_string()
    }
}

/// How a value is stored. Affects how it is written and how it is compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueKind {
    Dword,
    Qword,
    String,
    ExpandString,
    Binary,
}

/// What Windows has to be told after a tweak is written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Restart {
    #[default]
    None,
    Explorer,
    System,
}

impl Restart {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "no restart",
            Self::Explorer => "restart Explorer",
            Self::System => "restart Windows",
        }
    }
}

/// One registry value a tweak writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TweakEntry {
    pub hive: Hive,
    pub key: String,
    #[serde(default)]
    pub value_name: String,
    pub kind: ValueKind,
    /// What this tweak wants the value to be.
    pub recommended: String,
    /// What Windows ships with.
    ///
    /// Empty means "not restorable": there is no known previous state to go back to,
    /// so a tweak with no default on any entry cannot be reset.
    ///
    /// Two words are special, because a default is sometimes the *absence* of
    /// something rather than a number:
    /// `<deletevalue>` removes the value, `<delete>` removes the whole key.
    #[serde(default)]
    pub default: String,
}

impl TweakEntry {
    /// The value name as a human reads it. `(Default)` is what a registry editor
    /// calls the unnamed value, so that is what the data uses.
    pub fn display_name(&self) -> &str {
        if self.value_name.is_empty() {
            "(Default)"
        } else {
            &self.value_name
        }
    }

    /// Whether this entry has anything to restore.
    pub fn is_restorable(&self) -> bool {
        !self.default.trim().is_empty()
    }

    /// Whether the machine already satisfies this entry.
    ///
    /// The interesting case is a rule whose recommendation differs from the Windows
    /// default: then the recommendation is a *direction*, not an exact value, and a
    /// setting already past it counts as applied. Without that, a machine sitting at
    /// a stricter value than the tweak asks for would be reported as unfixed forever,
    /// and "Run" would quietly loosen it.
    pub fn is_applied(&self, current: Option<&str>) -> bool {
        let Some(current) = current else {
            return false;
        };
        if let (Some(now), Some(want), Some(ships)) = (
            number(current),
            number(&self.recommended),
            number(&self.default),
        ) && want != ships
        {
            return if want < ships { now <= want } else { now >= want };
        }
        current.eq_ignore_ascii_case(self.recommended.trim())
    }
}

/// A registry rule, as declared in the shipped catalogue or written by the user.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tweak {
    /// Empty in a file that omitted it; such an entry is dropped at load.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    /// The values this tweak writes. The first is the one the row reports. A rule
    /// with none is dropped at load, so this defaults rather than failing the file.
    #[serde(default)]
    pub entries: Vec<TweakEntry>,
    #[serde(default)]
    pub restart: Restart,
    /// Declared in the data. [`Tweak::needs_admin`] also infers it, so a file that
    /// forgets the flag cannot promise a write it has no right to make.
    #[serde(default)]
    pub requires_admin: bool,
    /// Ticked when the catalogue first loads.
    #[serde(default)]
    pub default_selected: bool,
    /// `None` for both Windows 10 and 11. `Some(11)` only where the setting exists
    /// on 11 alone, since offering a tweak that cannot land is worse than hiding it.
    #[serde(default)]
    pub os: Option<u8>,
}

/// What a scan found for one tweak.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TweakScan {
    pub status: TweakStatus,
    /// The first entry's current value, for the row and the inspector.
    pub current: Option<String>,
}

impl Default for TweakScan {
    fn default() -> Self {
        Self {
            status: TweakStatus::Unknown,
            current: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TweakStatus {
    /// Not scanned yet. Distinct from `NotApplied`, so the surface can say "press
    /// Analyze" instead of accusing the machine of being unfixed.
    #[default]
    Unknown,
    Applied,
    NotApplied,
}

impl TweakStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "not checked",
            Self::Applied => "already set",
            Self::NotApplied => "not set",
        }
    }
}

impl Tweak {
    /// Parses a catalogue, keeping only rules that can actually be applied.
    ///
    /// A *malformed* file is an error, because a JSON typo means the whole document
    /// is suspect. A well-formed file with one unusable entry is not: that entry is
    /// dropped and the rest of the catalogue still loads. The shipped catalogue is
    /// covered by a test, so a bad entry there fails the build instead of quietly
    /// shrinking the library.
    pub fn parse_library(text: &str) -> Result<Vec<Self>> {
        let tweaks: Vec<Self> =
            serde_json::from_str(crate::json::strip_bom(text)).map_err(|error| DebrisError::Catalog {
                detail: error.to_string(),
            })?;
        Ok(tweaks
            .into_iter()
            .filter(Self::is_valid)
            .map(|mut tweak| {
                tweak.requires_admin |= tweak.infer_admin();
                tweak
            })
            .collect())
    }

    /// A tweak is not a tweak without something to write. Enforced here rather than
    /// in each reader, so no tweak downstream of [`Tweak::parse_library`] can have
    /// entries that are missing.
    fn is_valid(&self) -> bool {
        !self.name.trim().is_empty() && !self.entries.is_empty()
    }

    /// Whether any entry lands somewhere a standard user cannot write. Policies keys
    /// need it too, even under `HKCU`: the key is read-only for the signed-in user.
    fn infer_admin(&self) -> bool {
        self.entries.iter().any(|entry| {
            entry.hive.needs_admin()
                || entry.key.to_ascii_lowercase().contains("\\policies\\")
                || entry
                    .key
                    .to_ascii_lowercase()
                    .starts_with("policies\\")
        })
    }

    /// Whether this tweak applies to the Windows build in question.
    pub fn applies_to(&self, build: u32) -> bool {
        match self.os {
            None => true,
            Some(11) => build >= 22000,
            Some(_) => build < 22000,
        }
    }

    /// Whether anything can be put back.
    pub fn is_restorable(&self) -> bool {
        self.entries.iter().any(TweakEntry::is_restorable)
    }

    /// The value the row reports: the first entry's.
    pub fn primary(&self) -> Option<&TweakEntry> {
        self.entries.first()
    }

    /// The word a row groups under, so an empty category does not erase a tweak.
    pub fn category_label(&self) -> &str {
        if self.category.trim().is_empty() {
            "Other"
        } else {
            &self.category
        }
    }

    /// Decides the status from a reading. Every entry has to be satisfied, not just
    /// the first: a two-part tweak that wrote one value and failed the other is not
    /// applied, and saying otherwise is how a half-done change goes unnoticed.
    pub fn scan(&self, mut read: impl FnMut(&TweakEntry) -> Option<String>) -> TweakScan {
        let mut status = TweakStatus::Applied;
        let mut current = None;
        for (index, entry) in self.entries.iter().enumerate() {
            let value = read(entry);
            if index == 0 {
                current = value.clone();
            }
            if !entry.is_applied(value.as_deref()) {
                status = TweakStatus::NotApplied;
            }
        }
        TweakScan { status, current }
    }

    /// A `.reg` document that puts the machine back where it is now.
    ///
    /// Only values that exist are exported, so restoring a backup can never invent a
    /// setting. Written in the UTF-16 `.reg` dialect Windows expects.
    pub fn backup_text(tweaks: &[Tweak], mut read: impl FnMut(&TweakEntry) -> Option<String>) -> String {
        let mut out = String::from("Windows Registry Editor Version 5.00\r\n\r\n");
        for tweak in tweaks {
            for entry in &tweak.entries {
                let Some(current) = read(entry) else {
                    continue;
                };
                out.push_str(&format!("[{}\\{}]\r\n", entry.hive.long(), entry.key));
                out.push_str(&reg_line(entry, &current));
                out.push_str("\r\n");
            }
        }
        out
    }
}

/// One `"name"=dword:00000000` line.
fn reg_line(entry: &TweakEntry, value: &str) -> String {
    let name = if entry.value_name.is_empty() {
        "@".to_string()
    } else {
        format!("\"{}\"", entry.value_name.replace('"', "\\\""))
    };
    match entry.kind {
        ValueKind::Dword => match number(value) {
            Some(parsed) => format!("{name}=dword:{:08x}\r\n", parsed as u32),
            None => string_line(&name, value),
        },
        ValueKind::Qword => match number(value) {
            Some(parsed) => format!(
                "{name}=hex(b):{}\r\n",
                parsed
                    .to_le_bytes()
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            None => string_line(&name, value),
        },
        ValueKind::Binary => {
            let bytes = hex_bytes(value);
            format!(
                "{name}=hex:{}\r\n",
                bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
        ValueKind::String | ValueKind::ExpandString => {
            let kind = if entry.kind == ValueKind::ExpandString {
                "hex(2):"
            } else {
                ""
            };
            if kind.is_empty() {
                string_line(&name, value)
            } else {
                // An expandable string is stored with a trailing NUL and is exported
                // as raw bytes, because `.reg` has no syntax for it.
                let bytes: Vec<u8> = value
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes)
                    .chain([0, 0])
                    .collect();
                format!(
                    "{name}=hex(2):{}\r\n",
                    bytes
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
        }
    }
}

fn string_line(name: &str, value: &str) -> String {
    format!(
        "{name}=\"{}\"\r\n",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    )
}

/// Reads a whole number, decimal or `0x`-prefixed.
pub fn number(value: &str) -> Option<u64> {
    let value = value.trim();
    match value.strip_prefix("0x").or_else(|| value.strip_prefix("0X")) {
        Some(hex) => u64::from_str_radix(hex, 16).ok(),
        None => value.parse().ok(),
    }
}

/// Reads a run of hex, ignoring whatever separators the data used.
pub fn hex_bytes(value: &str) -> Vec<u8> {
    let clean: String = value.chars().filter(char::is_ascii_hexdigit).collect();
    clean
        .as_bytes()
        .chunks(2)
        .filter(|chunk| chunk.len() == 2)
        .filter_map(|chunk| {
            u8::from_str_radix(std::str::from_utf8(chunk).ok()?, 16).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CATALOG: &str = r#"[
      {
        "name": "Disable File Explorer Ads",
        "category": "Ads",
        "description": "Turns off sync provider notifications.",
        "default_selected": true,
        "entries": [
          {
            "hive": "HKCU",
            "key": "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\Advanced",
            "value_name": "ShowSyncProviderNotifications",
            "kind": "dword",
            "recommended": "0",
            "default": "1"
          }
        ]
      },
      {
        "name": "Reduce Telemetry",
        "category": "Privacy",
        "entries": [
          { "hive": "HKEY_LOCAL_MACHINE", "key": "SOFTWARE\\Policies\\Microsoft\\Windows\\DataCollection",
            "value_name": "AllowTelemetry", "kind": "dword", "recommended": "1", "default": "3" }
        ]
      }
    ]"#;

    #[test]
    fn parses_both_hive_spellings() {
        let tweaks = Tweak::parse_library(CATALOG).unwrap();
        assert_eq!(tweaks.len(), 2);
        assert_eq!(tweaks[0].entries[0].hive, Hive::CurrentUser);
        assert_eq!(tweaks[1].entries[0].hive, Hive::LocalMachine);
    }

    #[test]
    fn a_policy_or_machine_key_demands_admin_even_when_the_file_forgot_to_say() {
        let tweaks = Tweak::parse_library(CATALOG).unwrap();
        assert!(!tweaks[0].requires_admin, "HKCU Explorer is a user setting");
        assert!(tweaks[1].requires_admin, "HKLM is inferred");
    }

    #[test]
    fn a_user_policies_key_also_demands_admin() {
        let text = r#"[{ "name": "x", "entries": [ { "hive": "HKCU",
            "key": "Software\\Policies\\Microsoft\\Windows\\Explorer",
            "value_name": "DisableSearchBoxSuggestions", "kind": "dword",
            "recommended": "1", "default": "0" } ] }]"#;
        assert!(Tweak::parse_library(text).unwrap()[0].requires_admin);
    }

    #[test]
    fn a_broken_catalogue_is_an_error_but_an_unusable_entry_is_only_dropped() {
        assert!(Tweak::parse_library("{ not json").is_err());
        // No entries, so there is nothing to write and nothing to offer.
        let kept = Tweak::parse_library(r#"[{"name":"x"},{"entries":[]},{"name":"","entries":[]}]"#)
            .unwrap();
        assert!(kept.is_empty());
    }

    #[test]
    fn an_exact_value_is_applied_only_when_it_matches() {
        let entry = TweakEntry {
            hive: Hive::CurrentUser,
            key: "k".into(),
            value_name: "v".into(),
            kind: ValueKind::Dword,
            recommended: "0".into(),
            default: "1".into(),
        };
        assert!(entry.is_applied(Some("0")));
        assert!(!entry.is_applied(Some("1")));
        assert!(!entry.is_applied(None), "an absent value is not applied");
    }

    #[test]
    fn a_lower_is_better_rule_is_satisfied_by_something_lower_still() {
        // Telemetry: 3 ships, 1 is wanted. Sitting at 0 is already stricter.
        let entry = TweakEntry {
            hive: Hive::LocalMachine,
            key: "k".into(),
            value_name: "v".into(),
            kind: ValueKind::Dword,
            recommended: "1".into(),
            default: "3".into(),
        };
        assert!(entry.is_applied(Some("0")), "stricter than asked for");
        assert!(entry.is_applied(Some("1")));
        assert!(!entry.is_applied(Some("2")));
    }

    #[test]
    fn a_higher_is_better_rule_is_satisfied_by_something_higher_still() {
        // Network throttling: 10 ships, "disabled" is 4294967295.
        let entry = TweakEntry {
            hive: Hive::LocalMachine,
            key: "k".into(),
            value_name: "v".into(),
            kind: ValueKind::Dword,
            recommended: "4294967295".into(),
            default: "10".into(),
        };
        assert!(entry.is_applied(Some("4294967295")));
        assert!(!entry.is_applied(Some("10")));
    }

    #[test]
    fn an_equal_recommendation_falls_back_to_an_exact_match() {
        // Recommended == default means the value is not a direction, so "already
        // better" is meaningless and only equality counts.
        let entry = TweakEntry {
            hive: Hive::CurrentUser,
            key: "k".into(),
            value_name: "v".into(),
            kind: ValueKind::Dword,
            recommended: "1".into(),
            default: "1".into(),
        };
        assert!(entry.is_applied(Some("1")));
        assert!(!entry.is_applied(Some("0")));
    }

    fn two_entry_tweak() -> Tweak {
        Tweak {
            name: "Two".into(),
            category: "Test".into(),
            description: String::new(),
            entries: vec![
                TweakEntry {
                    hive: Hive::CurrentUser,
                    key: "a".into(),
                    value_name: "first".into(),
                    kind: ValueKind::Dword,
                    recommended: "0".into(),
                    default: "1".into(),
                },
                TweakEntry {
                    hive: Hive::CurrentUser,
                    key: "b".into(),
                    value_name: "second".into(),
                    kind: ValueKind::Dword,
                    recommended: "1".into(),
                    default: "0".into(),
                },
            ],
            restart: Restart::None,
            requires_admin: false,
            default_selected: false,
            os: None,
        }
    }

    #[test]
    fn every_entry_must_match_before_a_tweak_counts_as_applied() {
        let tweak = two_entry_tweak();
        let both = tweak.scan(|_| Some("0".to_string()));
        assert_eq!(both.status, TweakStatus::NotApplied, "the second entry is unmet");

        let applied = tweak.scan(|entry| {
            Some(if entry.value_name == "first" { "0" } else { "1" }.to_string())
        });
        assert_eq!(applied.status, TweakStatus::Applied);
    }

    #[test]
    fn a_scan_reports_the_first_entrys_value() {
        let tweak = two_entry_tweak();
        let scan = tweak.scan(|entry| {
            Some(if entry.value_name == "first" { "7" } else { "9" }.to_string())
        });
        assert_eq!(scan.current.as_deref(), Some("7"));
    }

    #[test]
    fn the_os_filter_hides_what_cannot_land() {
        let mut tweak = two_entry_tweak();
        tweak.os = Some(11);
        assert!(tweak.applies_to(26100), "Windows 11");
        assert!(!tweak.applies_to(19045), "Windows 10");
        tweak.os = Some(10);
        assert!(tweak.applies_to(19045));
        assert!(!tweak.applies_to(26100));
        tweak.os = None;
        assert!(tweak.applies_to(19045) && tweak.applies_to(26100));
    }

    #[test]
    fn only_a_tweak_with_a_declared_default_can_be_restored() {
        let mut tweak = two_entry_tweak();
        assert!(tweak.is_restorable());
        tweak.entries[0].default = String::new();
        tweak.entries[1].default = "<deletevalue>".into();
        assert!(tweak.is_restorable(), "deleting is a way back too");
        tweak.entries[1].default = String::new();
        assert!(!tweak.is_restorable());
    }

    #[test]
    fn a_backup_exports_only_what_actually_exists() {
        let tweak = two_entry_tweak();
        let text = Tweak::backup_text(&[tweak], |entry| {
            (entry.value_name == "first").then(|| "3".to_string())
        });
        assert!(text.starts_with("Windows Registry Editor Version 5.00"));
        assert!(text.contains("[HKEY_CURRENT_USER\\a]"));
        assert!(text.contains("\"first\"=dword:00000003"));
        assert!(
            !text.contains("second"),
            "a value that is not set has nothing to back up: {text}"
        );
    }

    #[test]
    fn a_backup_writes_the_default_value_as_at_symbol() {
        let entry = TweakEntry {
            hive: Hive::CurrentUser,
            key: "Software\\CLASSES\\CLSID\\{abc}\\InprocServer32".into(),
            value_name: String::new(),
            kind: ValueKind::String,
            recommended: String::new(),
            default: "<delete>".into(),
        };
        let tweak = Tweak {
            name: "Classic menus".into(),
            category: "Appearance".into(),
            description: String::new(),
            entries: vec![entry],
            restart: Restart::Explorer,
            requires_admin: false,
            default_selected: false,
            os: Some(11),
        };
        let text = Tweak::backup_text(&[tweak], |_| Some(String::new()));
        assert!(text.contains("@=\"\""), "{text}");
    }

    #[test]
    fn numbers_parse_both_ways_and_hex_bytes_ignore_separators() {
        assert_eq!(number("4294967295"), Some(4294967295));
        assert_eq!(number("0x10"), Some(16));
        assert_eq!(number("nope"), None);
        assert_eq!(hex_bytes("0A-FF 01"), vec![0x0a, 0xff, 0x01]);
    }
}

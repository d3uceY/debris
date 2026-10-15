//! The one thing every JSON entry point needs to do before `serde_json` sees it.
//!
//! `serde_json` rejects a byte-order mark, and a BOM is easy to acquire by accident:
//! PowerShell 5.1's `Set-Content -Encoding UTF8` writes one, older Notepad writes one,
//! and so do a lot of editors on the "save as UTF-8" path. A file that looks perfectly
//! correct in an editor would otherwise read as *empty* — the worst kind of failure,
//! because `Settings::parse` falls back to the defaults and the app silently forgets
//! everything the user configured.
//!
//! So it is stripped in one place, and every catalogue and settings reader goes
//! through it.

/// A string with any leading byte-order mark removed.
pub fn strip_bom(text: &str) -> &str {
    text.strip_prefix('\u{feff}').unwrap_or(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_byte_order_mark_is_removed() {
        assert_eq!(strip_bom("\u{feff}{}"), "{}");
    }

    #[test]
    fn text_without_one_is_untouched() {
        assert_eq!(strip_bom("{}"), "{}");
        assert_eq!(strip_bom(""), "");
    }

    #[test]
    fn only_a_leading_mark_is_removed() {
        // A mark inside a JSON *string* is content, and it stays.
        assert_eq!(strip_bom("[\"\u{feff}\"]"), "[\"\u{feff}\"]");
    }
}

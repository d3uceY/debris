/// A named set of package names — "Fresh Start", or one the user saved.
///
/// The format is deliberately the plainest thing that works: one package name per
/// line, `#` for comments, blank lines ignored. A diffable text file is a better
/// citizen in a repository and needs no parser.
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
}

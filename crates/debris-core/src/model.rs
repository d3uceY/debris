/// One Appx/UWP package, as reported by `Get-AppxPackage`.
///
/// `name` is the identity the user reads and the identity we stage by; `full_name`
/// is what an uninstall is actually keyed on. Keeping both means the ledger can
/// display something legible and still act on something exact.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AppxPackage {
    /// e.g. `Microsoft.BingNews`
    pub name: String,
    /// e.g. `Microsoft.BingNews_4.12.0.0_x64__8wekyb3d8bbwe`
    pub full_name: String,
    /// The raw publisher distinguished name.
    pub publisher: String,
    /// e.g. `4.12.0.0`
    pub version: String,
    /// Present in the system catalog — hidden by default, flagged in a plan.
    pub is_system: bool,
}

impl AppxPackage {
    pub fn new(name: impl Into<String>, full_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            full_name: full_name.into(),
            publisher: String::new(),
            version: String::new(),
            is_system: false,
        }
    }

    /// Marks the package as system-critical if the catalog recognises it.
    /// Chainable so a parse loop reads as one statement.
    pub fn classify(mut self, is_system: bool) -> Self {
        self.is_system = is_system;
        self
    }

    /// `CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US`
    /// becomes `Microsoft Corporation`. Falls back to the raw value when the shape
    /// is unexpected — a tool like this must never panic on a weird publisher string.
    pub fn publisher_short(&self) -> &str {
        let after_cn = self
            .publisher
            .strip_prefix("CN=")
            .or_else(|| self.publisher.split("CN=").nth(1))
            .unwrap_or(&self.publisher);
        after_cn.split(',').next().unwrap_or(after_cn).trim()
    }

    /// The line the UI shows under the name. Built here, not in the view, so the
    /// backend and the UI can never disagree about it.
    pub fn meta_line(&self) -> String {
        match (self.publisher_short(), self.version.is_empty()) {
            ("", true) => String::new(),
            ("", false) => self.version.clone(),
            (publisher, true) => publisher.to_string(),
            (publisher, false) => format!("{publisher}  ·  {version}", version = self.version),
        }
    }
}

impl Default for AppxPackage {
    fn default() -> Self {
        Self::new(String::new(), String::new())
    }
}

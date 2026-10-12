use crate::model::AppxPackage;

/// The two numbers in the header of the ledger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub installed: usize,
    pub staged: usize,
}

impl Counts {
    pub fn is_dirty(&self) -> bool {
        self.staged > 0
    }
}

/// What the inventory column is currently showing.
#[derive(Clone, Debug, Default)]
pub struct Filter {
    pub query: String,
    pub show_system: bool,
}

impl Filter {
    pub fn matches(&self, package: &AppxPackage) -> bool {
        if package.is_system && !self.show_system {
            return false;
        }
        let query = self.query.trim();
        if query.is_empty() {
            return true;
        }
        contains_ci(&package.name, query) || contains_ci(package.publisher_short(), query)
    }
}

/// ASCII case-insensitive substring search that does not allocate.
///
/// The obvious `haystack.to_lowercase().contains(..)` allocates twice per row, and
/// this runs over every visible row on every keystroke. Worth the eight lines.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    match n.len() {
        0 => true,
        len if len > h.len() => false,
        len => h.windows(len).any(|window| window.eq_ignore_ascii_case(n)),
    }
}

/// The ledger: what stays, and what is staged to go.
///
/// Staging is by **package name**, matching what `Get-AppxPackage <name>` accepts.
/// A name can resolve to more than one installed package (architectures, versions),
/// so staging moves every package that shares the name and the plan lists each one.
///
/// `visible` is memoized rather than recomputed per call: it is read once per frame
/// by the inventory list, and rebuilt only when the inventory or the filter changes.
#[derive(Clone, Debug, Default)]
pub struct Staging {
    inventory: Vec<AppxPackage>,
    removal: Vec<AppxPackage>,
    filter: Filter,
    visible: Vec<usize>,
}

impl Staging {
    pub fn new(packages: Vec<AppxPackage>) -> Self {
        let mut staging = Self::default();
        staging.set_inventory(packages);
        staging
    }

    /// Replaces the inventory, keeping staged packages that are still installed.
    /// Anything staged but no longer present is dropped — a stale entry in the
    /// removal column would be a lie about what is about to happen.
    pub fn set_inventory(&mut self, mut packages: Vec<AppxPackage>) {
        packages.sort_by(|a, b| a.name.cmp(&b.name).then(a.full_name.cmp(&b.full_name)));
        let present: Vec<String> = packages.iter().map(|p| p.name.clone()).collect();
        self.removal
            .retain(|staged| present.iter().any(|name| name == &staged.name));
        self.inventory = packages;
        self.rebuild_visible();
    }

    pub fn inventory(&self) -> &[AppxPackage] {
        &self.inventory
    }

    pub fn removal(&self) -> &[AppxPackage] {
        &self.removal
    }

    /// Indices into [`Staging::inventory`] that pass the current filter.
    pub fn visible(&self) -> &[usize] {
        &self.visible
    }

    pub fn visible_len(&self) -> usize {
        self.visible.len()
    }

    pub fn visible_package(&self, row: usize) -> Option<&AppxPackage> {
        self.visible
            .get(row)
            .and_then(|index| self.inventory.get(*index))
    }

    pub fn counts(&self) -> Counts {
        Counts {
            installed: self.inventory.len(),
            staged: self.removal.len(),
        }
    }

    pub fn filter(&self) -> &Filter {
        &self.filter
    }

    pub fn query(&self) -> &str {
        &self.filter.query
    }

    pub fn set_query(&mut self, query: impl Into<String>) {
        let query = query.into();
        if self.filter.query != query {
            self.filter.query = query;
            self.rebuild_visible();
        }
    }

    pub fn show_system(&self) -> bool {
        self.filter.show_system
    }

    pub fn set_show_system(&mut self, show: bool) {
        if self.filter.show_system != show {
            self.filter.show_system = show;
            self.rebuild_visible();
        }
    }

    pub fn is_staged(&self, name: &str) -> bool {
        self.removal.iter().any(|package| package.name == name)
    }

    /// Moves every package sharing `name` from the inventory into the removal
    /// column. Returns how many moved.
    pub fn stage(&mut self, name: &str) -> usize {
        if name.is_empty() {
            return 0;
        }
        let mut moved = Vec::new();
        let mut kept = Vec::with_capacity(self.inventory.len());
        for package in std::mem::take(&mut self.inventory) {
            if package.name == name {
                moved.push(package);
            } else {
                kept.push(package);
            }
        }
        self.inventory = kept;
        let count = moved.len();
        if count > 0 {
            self.removal.extend(moved);
            self.sort_removal();
            self.rebuild_visible();
        }
        count
    }

    /// Returns every package sharing `name` to the inventory.
    pub fn unstage(&mut self, name: &str) -> usize {
        if name.is_empty() {
            return 0;
        }
        let mut moved = Vec::new();
        let mut kept = Vec::with_capacity(self.removal.len());
        for package in std::mem::take(&mut self.removal) {
            if package.name == name {
                moved.push(package);
            } else {
                kept.push(package);
            }
        }
        self.removal = kept;
        let count = moved.len();
        if count > 0 {
            self.inventory.extend(moved);
            self.inventory.sort_by(|a, b| a.name.cmp(&b.name).then(a.full_name.cmp(&b.full_name)));
            self.rebuild_visible();
        }
        count
    }

    /// Stages everything currently visible — which is what the user sees when they
    /// press "Add all", not the whole inventory behind a filter.
    ///
    /// One pass, not a `stage()` per row. Calling `stage()` in a loop re-scanned the
    /// whole inventory and rebuilt the visible index every time, which made this
    /// quadratic: 5,000 packages took 330ms in release. It is now a partition.
    pub fn stage_all_visible(&mut self) -> usize {
        if self.visible.is_empty() {
            return 0;
        }

        // `inventory` is iterated in index order and `visible` is ascending, so one
        // cursor walks both.
        let visible = std::mem::take(&mut self.visible);
        let mut cursor = 0usize;
        let mut moved = Vec::new();
        let mut kept = Vec::with_capacity(self.inventory.len().saturating_sub(visible.len()));

        for (index, package) in std::mem::take(&mut self.inventory).into_iter().enumerate() {
            if cursor < visible.len() && visible[cursor] == index {
                cursor += 1;
                moved.push(package);
            } else {
                kept.push(package);
            }
        }

        self.inventory = kept;
        let count = moved.len();
        if count > 0 {
            self.removal.extend(moved);
            self.sort_removal();
        }
        self.rebuild_visible();
        count
    }

    pub fn unstage_all(&mut self) -> usize {
        let count = self.removal.len();
        if count == 0 {
            return 0;
        }
        self.inventory.append(&mut self.removal);
        self.inventory.sort_by(|a, b| a.name.cmp(&b.name).then(a.full_name.cmp(&b.full_name)));
        self.rebuild_visible();
        count
    }

    /// Loads a preset into the removal column, replacing what is staged.
    /// Names that are not installed are ignored, and the caller is told how many
    /// landed — a preset is a request, not a guarantee.
    ///
    /// Like `stage_all_visible`, this is one partition rather than a `stage()` per
    /// matching name.
    pub fn load_removal(&mut self, names: &[String]) -> usize {
        self.unstage_all();

        let wanted: std::collections::HashSet<String> = names
            .iter()
            .map(|name| name.trim().to_ascii_lowercase())
            .filter(|name| !name.is_empty())
            .collect();

        if wanted.is_empty() {
            self.rebuild_visible();
            return 0;
        }

        let mut moved = Vec::new();
        let mut kept = Vec::with_capacity(self.inventory.len());
        for package in std::mem::take(&mut self.inventory) {
            if wanted.contains(&package.name.to_ascii_lowercase()) {
                moved.push(package);
            } else {
                kept.push(package);
            }
        }

        self.inventory = kept;
        let count = moved.len();
        if count > 0 {
            self.removal.extend(moved);
            self.sort_removal();
        }
        self.rebuild_visible();
        count
    }

    pub fn removal_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.removal.iter().map(|p| p.name.clone()).collect();
        names.dedup();
        names
    }

    fn sort_removal(&mut self) {
        self.removal
            .sort_by(|a, b| a.name.cmp(&b.name).then(a.full_name.cmp(&b.full_name)));
    }

    fn rebuild_visible(&mut self) {
        self.visible = self
            .inventory
            .iter()
            .enumerate()
            .filter(|(_, package)| self.filter.matches(package))
            .map(|(index, _)| index)
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(name: &str, system: bool) -> AppxPackage {
        AppxPackage::new(name, format!("{name}_1.0.0.0_x64__abc")).classify(system)
    }

    fn sample() -> Vec<AppxPackage> {
        vec![
            package("Microsoft.BingNews", false),
            package("Microsoft.WindowsStore", true),
            package("SpotifyAB.SpotifyMusic", false),
        ]
    }

    #[test]
    fn system_packages_are_hidden_until_asked_for() {
        let staging = Staging::new(sample());
        assert_eq!(staging.visible_len(), 2);
        assert!(staging.visible_package(0).map(|p| p.name.as_str()) != Some("Microsoft.WindowsStore"));
    }

    #[test]
    fn showing_system_packages_reveals_them() {
        let mut staging = Staging::new(sample());
        staging.set_show_system(true);
        assert_eq!(staging.visible_len(), 3);
    }

    #[test]
    fn visible_is_sorted_by_name() {
        let staging = Staging::new(sample());
        assert_eq!(staging.visible_package(0).unwrap().name, "Microsoft.BingNews");
        assert_eq!(staging.visible_package(1).unwrap().name, "SpotifyAB.SpotifyMusic");
    }

    #[test]
    fn query_filters_case_insensitively() {
        let mut staging = Staging::new(sample());
        staging.set_query("spotify");
        assert_eq!(staging.visible_len(), 1);
        assert_eq!(staging.visible_package(0).unwrap().name, "SpotifyAB.SpotifyMusic");
    }

    #[test]
    fn query_matches_publisher() {
        let mut packages = sample();
        packages[0].publisher = "CN=Microsoft Corporation, O=Microsoft".into();
        let mut staging = Staging::new(packages);
        staging.set_query("microsoft corporation");
        assert_eq!(staging.visible_len(), 1);
    }

    #[test]
    fn whitespace_only_query_matches_everything() {
        let mut staging = Staging::new(sample());
        staging.set_query("   ");
        assert_eq!(staging.visible_len(), 2);
    }

    #[test]
    fn staging_moves_the_package_between_columns() {
        let mut staging = Staging::new(sample());
        assert_eq!(staging.stage("Microsoft.BingNews"), 1);
        assert_eq!(staging.counts(), Counts { installed: 2, staged: 1 });
        assert!(staging.is_staged("Microsoft.BingNews"));
        assert_eq!(staging.unstage("Microsoft.BingNews"), 1);
        assert_eq!(staging.counts(), Counts { installed: 3, staged: 0 });
    }

    #[test]
    fn staging_an_absent_package_is_a_no_op() {
        let mut staging = Staging::new(sample());
        assert_eq!(staging.stage("Nothing.Here"), 0);
        assert_eq!(staging.counts(), Counts { installed: 3, staged: 0 });
    }

    #[test]
    fn staging_an_empty_name_is_a_no_op() {
        let mut staging = Staging::new(sample());
        assert_eq!(staging.stage(""), 0);
        assert_eq!(staging.unstage(""), 0);
    }

    #[test]
    fn a_name_with_several_packages_moves_all_of_them() {
        let packages = vec![
            package("Microsoft.BingNews", false),
            AppxPackage::new("Microsoft.BingNews", "Microsoft.BingNews_x86__abc"),
        ];
        let mut staging = Staging::new(packages);
        assert_eq!(staging.stage("Microsoft.BingNews"), 2);
        assert_eq!(staging.counts(), Counts { installed: 0, staged: 2 });
    }

    #[test]
    fn stage_all_visible_respects_the_filter() {
        let mut staging = Staging::new(sample());
        staging.set_query("microsoft");
        assert_eq!(staging.stage_all_visible(), 1);
        assert!(staging.is_staged("Microsoft.BingNews"));
        assert!(!staging.is_staged("SpotifyAB.SpotifyMusic"));
    }

    #[test]
    fn unstage_all_returns_everything() {
        let mut staging = Staging::new(sample());
        staging.stage("Microsoft.BingNews");
        staging.stage("SpotifyAB.SpotifyMusic");
        assert_eq!(staging.unstage_all(), 2);
        assert_eq!(staging.counts(), Counts { installed: 3, staged: 0 });
        // and the inventory is still sorted
        assert_eq!(staging.visible_package(0).unwrap().name, "Microsoft.BingNews");
    }

    #[test]
    fn refresh_drops_staged_packages_that_vanished() {
        let mut staging = Staging::new(sample());
        staging.stage("SpotifyAB.SpotifyMusic");
        staging.set_inventory(vec![package("Microsoft.BingNews", false)]);
        assert_eq!(staging.counts().staged, 0);
    }

    #[test]
    fn refresh_keeps_staged_packages_that_remain() {
        let mut staging = Staging::new(sample());
        staging.stage("SpotifyAB.SpotifyMusic");
        staging.set_inventory(sample());
        assert_eq!(staging.counts().staged, 1);
    }

    #[test]
    fn presets_load_into_the_removal_column_and_ignore_missing_names() {
        let mut staging = Staging::new(sample());
        let loaded = staging.load_removal(&[
            "Microsoft.BingNews".into(),
            "Not.Installed".into(),
            "spotifyab.spotifymusic".into(),
        ]);
        assert_eq!(loaded, 2);
        assert_eq!(staging.counts().staged, 2);
    }

    #[test]
    fn loading_a_preset_replaces_the_previous_selection() {
        let mut staging = Staging::new(sample());
        staging.stage("SpotifyAB.SpotifyMusic");
        staging.load_removal(&["Microsoft.BingNews".into()]);
        assert_eq!(staging.counts().staged, 1);
        assert!(staging.is_staged("Microsoft.BingNews"));
    }

    #[test]
    fn contains_ci_handles_the_edges() {
        assert!(contains_ci("Microsoft", ""));
        assert!(contains_ci("Microsoft", "cros"));
        assert!(contains_ci("Microsoft", "MICRO"));
        assert!(!contains_ci("Microsoft", "Zune"));
        assert!(!contains_ci("short", "much longer needle"));
    }
}

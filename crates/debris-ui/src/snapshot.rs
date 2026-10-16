//! The one value the interface draws from.
//!
//! A `Snapshot` is plain data on purpose. It is what the app hands the views, so a
//! view can be laid out for any state — loading, empty, denied — without knowing
//! where that state came from. The `Arc`s are the memoized derived data the
//! performance rule asks for: the filtered inventory is rebuilt when the filter
//! changes, not once per frame, and cloning an `Arc` per frame is free.
//!
//! Shell geometry lives here too (`sidebar_open`, `inspector_open`). Those two bits
//! are the *only* view state the app owns; everything else about arrangement is
//! decided inside the views from the data below.

use crate::icons::Icon;
use debris_core::{
    AppxPackage, BloatCatalog, Counts, Extension, Preset, RemovalPlan, ScriptMeta, Tweak, TweakScan,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Packages,
    Fixers,
    Scripts,
    Extensions,
    Presets,
}

impl Screen {
    pub const ALL: [Screen; 5] = [
        Screen::Packages,
        Screen::Fixers,
        Screen::Scripts,
        Screen::Extensions,
        Screen::Presets,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Screen::Packages => "Packages",
            Screen::Fixers => "Fixers",
            Screen::Scripts => "Scripts",
            Screen::Extensions => "Extensions",
            Screen::Presets => "Presets",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Screen::Packages => Icon::Package,
            Screen::Fixers => Icon::Wrench,
            Screen::Scripts => Icon::Terminal,
            Screen::Extensions => Icon::Puzzle,
            Screen::Presets => Icon::Layers,
        }
    }

    /// What the surface *contains*, as opposed to what the nav calls it. The rail
    /// says "Packages"; the panel says "Installed packages". Two names for two jobs:
    /// one is a place you can be, the other is a thing you can read.
    pub fn title(self) -> &'static str {
        match self {
            Screen::Packages => "Installed packages",
            Screen::Fixers => "Registry fixers",
            Screen::Scripts => "Script library",
            Screen::Extensions => "Extensions",
            Screen::Presets => "Saved presets",
        }
    }

    /// The placeholder in the title bar's command shell while this surface is up.
    pub fn hint(self) -> &'static str {
        match self {
            Screen::Packages => "Filter packages by name or publisher…",
            Screen::Fixers => "Filter fixers…",
            Screen::Scripts => "Filter the library…",
            Screen::Extensions => "Filter extensions…",
            Screen::Presets => "Find a preset…",
        }
    }
}

/// One extension, and the catalogue it came from.
///
/// Two catalogues can legitimately list the same script. Keeping the source attached
/// means the list can say where an entry came from instead of presenting a merged blob
/// as if it were one publisher's work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogEntry {
    pub source: String,
    pub extension: Extension,
}

impl CatalogEntry {
    /// The identity used for install state, which is the URL the script comes from.
    pub fn url(&self) -> &str {
        &self.extension.url
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusTone {
    Neutral,
    Caution,
    Bad,
}

/// An action the user has asked for once and not yet confirmed.
///
/// This is the only thing that turns a click into a change. Staging a package is a
/// proposal; removing it, or running a script that writes, waits here until the user
/// says yes a second time. The variant carries identity only — the copy is composed
/// from the rest of the snapshot (`plan`, `scripts`) so there is one source of truth.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pending {
    /// Remove the packages currently staged in the queue.
    Removal,
    /// Run every ticked script.
    Scripts,
    /// Write every ticked tweak's recommended value.
    Fixers,
    /// Put every ticked tweak back to the value Windows ships with.
    Restore,
    /// Run one installed extension, by the URL it was installed from.
    Extension(String),
}

/// The bottom edge. One line, always in the same place — never a toast that
/// disappears before it is read.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Idle,
    /// What just happened.
    Info(String),
    /// Something is running.
    Activity(String),
    /// A caveat the user should read.
    Caution(String),
    /// A failure, naming the problem.
    Error(String),
}

impl Status {
    pub fn text(&self) -> &str {
        match self {
            Status::Idle => "Ready",
            Status::Info(text)
            | Status::Activity(text)
            | Status::Caution(text)
            | Status::Error(text) => text,
        }
    }

    pub fn tone(&self) -> StatusTone {
        match self {
            Status::Idle | Status::Info(_) => StatusTone::Neutral,
            Status::Activity(_) => StatusTone::Neutral,
            Status::Caution(_) => StatusTone::Caution,
            Status::Error(_) => StatusTone::Bad,
        }
    }

    pub fn is_busy(&self) -> bool {
        matches!(self, Status::Activity(_))
    }
}

/// Everything a frame needs.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub screen: Screen,

    pub query: String,
    /// Ctrl+A in the search field: the whole value is selected, so the next key
    /// replaces it. Every edit clears it, which is why nothing else has to know.
    pub query_selected: bool,
    pub show_system: bool,

    /// The filtered inventory, memoized. `Staging::visible()` indexes this exactly.
    pub inventory: Arc<Vec<AppxPackage>>,
    /// The removal queue — the left column, on every surface.
    pub staged: Arc<Vec<AppxPackage>>,
    /// The staged names as a set, memoized by the app. A virtualized list asks this
    /// question once per visible row per frame; a linear scan of `staged` would turn
    /// "stage all 5,000" into a quadratic frame.
    pub staged_names: Arc<HashSet<String>>,
    pub counts: Counts,

    /// What the confirm step would do.
    pub plan: RemovalPlan,

    pub scripts: Arc<Vec<ScriptMeta>>,
    /// The ticked scripts. The library runs a marked set, not one row at a time.
    pub script_selection: Arc<HashSet<String>>,
    pub presets: Arc<Vec<Preset>>,

    /// The fixer catalogue, and the rules ticked in it.
    pub tweaks: Arc<Vec<Tweak>>,
    pub tweak_selection: Arc<HashSet<String>>,
    /// What the last Analyze pass found, keyed by tweak name. A name that is absent
    /// has not been checked, which is why [`TweakStatus::Unknown`] exists.
    pub tweak_scans: Arc<HashMap<String, TweakScan>>,
    /// Whether a batch of fixers writes a `.reg` backup first.
    pub backup_before_apply: bool,

    /// What the catalogue says each package is. Advisory: it describes, never gates.
    pub bloat: Arc<BloatCatalog>,

    /// Extensions, gathered from every catalogue the user has added.
    pub extensions: Arc<Vec<CatalogEntry>>,
    pub extension_sources: Arc<Vec<String>>,
    /// Script URLs already downloaded, and the version recorded when they were.
    pub installed_extensions: Arc<HashMap<String, String>>,
    /// What the user is typing into the catalogue field, before it is added.
    pub source_input: String,
    /// Ctrl+A in that field.
    pub source_selected: bool,

    /// The keyboard cursor into `inventory`. Operate surfaces are keyboard-first.
    pub cursor: usize,
    /// Whether a scan has completed at least once, so "empty" can be distinguished
    /// from "not looked yet".
    pub scanned: bool,
    /// `None` until the elevation probe has run. Assuming `false` would show
    /// "Standard user" to an administrator for the first few seconds.
    pub elevated: Option<bool>,

    /// A destructive action awaiting its second, explicit confirmation. `None` means
    /// no dialog is up.
    pub pending: Option<Pending>,

    /// The left column: on by default, because the removal queue is the point of the
    /// app and hiding it hides the thing the user came to check.
    pub sidebar_open: bool,
    /// The right column: off by default, because it is a second opinion rather than
    /// the main event.
    pub inspector_open: bool,

    pub status: Status,
    /// A scan in flight — the surface shows the determinate bar rather than nothing.
    pub loading: Option<String>,
    /// A scan or a run failed in a way that has no inline home.
    pub error: Option<String>,
}

impl Snapshot {
    pub fn new() -> Self {
        Self {
            sidebar_open: true,
            inspector_open: false,
            ..Default::default()
        }
    }

    pub fn inventory_len(&self) -> usize {
        self.inventory.len()
    }

    /// The package under the keyboard cursor — what the inspector describes.
    pub fn focused(&self) -> Option<&AppxPackage> {
        self.inventory.get(self.cursor)
    }

    pub fn clamp_cursor(&mut self) {
        if self.inventory.is_empty() {
            self.cursor = 0;
        } else if self.cursor >= self.inventory.len() {
            self.cursor = self.inventory.len() - 1;
        }
    }

    /// The inventory is legitimately empty in two different ways, and they need
    /// different copy.
    pub fn inventory_is_empty(&self) -> bool {
        self.inventory.is_empty()
    }

    pub fn filter_is_active(&self) -> bool {
        !self.query.trim().is_empty()
    }

    pub fn is_busy(&self) -> bool {
        self.loading.is_some()
    }

    /// The ticked tweaks, in catalogue order so a run reads top to bottom.
    pub fn selected_tweaks(&self) -> Vec<&Tweak> {
        self.tweaks
            .iter()
            .filter(|tweak| self.tweak_selection.contains(&tweak.name))
            .collect()
    }

    /// The scan for one tweak, or "not checked yet".
    pub fn tweak_scan(&self, name: &str) -> TweakScan {
        self.tweak_scans.get(name).cloned().unwrap_or_default()
    }

    /// The ticked scripts, in library order.
    pub fn selected_scripts(&self) -> Vec<&ScriptMeta> {
        self.scripts
            .iter()
            .filter(|script| self.script_selection.contains(&script.id))
            .collect()
    }

    /// Whether a script has been ticked.
    pub fn script_is_selected(&self, id: &str) -> bool {
        self.script_selection.contains(id)
    }
}

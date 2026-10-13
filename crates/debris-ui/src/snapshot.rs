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
use debris_core::{AppxPackage, Counts, Preset, RemovalPlan, ScriptMeta};
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Packages,
    Scripts,
    Presets,
}

impl Screen {
    pub const ALL: [Screen; 3] = [Screen::Packages, Screen::Scripts, Screen::Presets];

    pub fn label(self) -> &'static str {
        match self {
            Screen::Packages => "Packages",
            Screen::Scripts => "Scripts",
            Screen::Presets => "Presets",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Screen::Packages => Icon::Package,
            Screen::Scripts => Icon::Terminal,
            Screen::Presets => Icon::Layers,
        }
    }

    /// What the surface *contains*, as opposed to what the nav calls it. The rail
    /// says "Packages"; the panel says "Installed packages". Two names for two jobs:
    /// one is a place you can be, the other is a thing you can read.
    pub fn title(self) -> &'static str {
        match self {
            Screen::Packages => "Installed packages",
            Screen::Scripts => "Script library",
            Screen::Presets => "Saved presets",
        }
    }

    /// The line under the title, saying what the surface is for.
    pub fn blurb(self) -> &'static str {
        match self {
            Screen::Packages => {
                "Everything installed for this user. Stage what should go, then review before it runs."
            }
            Screen::Scripts => {
                "Audited PowerShell from the local library. Nothing runs unless you run it."
            }
            Screen::Presets => "Named sets of packages, including the Fresh Start list.",
        }
    }

    /// The placeholder in the title bar's command shell while this surface is up.
    pub fn hint(self) -> &'static str {
        match self {
            Screen::Packages => "Filter packages by name or publisher…",
            Screen::Scripts => "Filter the library…",
            Screen::Presets => "Find a preset…",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusTone {
    Neutral,
    Good,
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
    /// Run one script, by id.
    Script(String),
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
    pub presets: Arc<Vec<Preset>>,

    /// The keyboard cursor into `inventory`. Operate surfaces are keyboard-first.
    pub cursor: usize,
    /// Whether a scan has completed at least once, so "empty" can be distinguished
    /// from "not looked yet".
    pub scanned: bool,
    /// `None` until the elevation probe has run. Assuming `false` would show
    /// "Standard user" to an administrator for the first few seconds.
    pub elevated: Option<bool>,
    /// Scripts are previewed unless the user turns this off.
    pub dry_run: bool,

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
}

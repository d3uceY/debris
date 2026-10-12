//! The one value the interface draws from.
//!
//! A `Snapshot` is plain data on purpose. It is what the app hands the views, so a
//! view can be laid out for any state — loading, empty, denied — without knowing
//! where that state came from. The two `Arc`s are the memoized derived data the
//! skill's performance rule asks for: the filtered inventory is rebuilt when the
//! filter changes, not once per frame, and cloning an `Arc` per frame is free.

use crate::icons::Icon;
use debris_core::{AppxPackage, Counts, Preset, RemovalPlan, ScriptMeta};
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
            Screen::Scripts => Icon::Script,
            Screen::Presets => Icon::Layers,
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusTone {
    Neutral,
    Good,
    Caution,
    Bad,
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

}

/// Everything a frame needs.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub screen: Screen,

    pub query: String,
    pub show_system: bool,

    /// The filtered inventory, memoized. `Staging::visible()` indexes this exactly.
    pub inventory: Arc<Vec<AppxPackage>>,
    /// The removal column.
    pub staged: Arc<Vec<AppxPackage>>,
    /// The staged names as a set, memoized by the app. A virtualized list asks this
    /// question once per visible row per frame; a linear scan of `staged` would turn
    /// "stage all 5,000" into a quadratic frame.
    pub staged_names: Arc<std::collections::HashSet<String>>,
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

    pub status: Status,
    /// A scan in flight — the surface shows the determinate bar rather than nothing.
    pub loading: Option<String>,
    /// Mirrors `Theme::reduce_motion`, so a handler can read it without reaching for
    /// the global.
    pub reduce_motion: bool,
    /// A scan or a run failed in a way that has no inline home.
    pub error: Option<String>,
}

impl Snapshot {
    pub fn inventory_len(&self) -> usize {
        self.inventory.len()
    }

    pub fn cursor_row(&self) -> Option<&AppxPackage> {
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
}

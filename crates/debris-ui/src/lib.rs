//! The GPUI surface for debris.
//!
//! This crate knows about [`debris_core`] and GPUI, and nothing else. It does not
//! spawn a process, read a file, or know what PowerShell is. Behaviour arrives
//! through the [`DebrisApp`] trait, which the binary implements — which is what lets
//! the whole interface be laid out and reasoned about without an operating system
//! underneath it.
//!
//! The design system lives in [`theme`], the drawn icon set in [`icons`], the
//! reusable pieces in [`components`], and the composition in [`views`]. Read
//! `DESIGN.md` at the repository root before changing a token: the deviations from
//! the usual playbook (square corners, no shadows) are deliberate.

pub mod components;
pub mod icons;
pub mod snapshot;
pub mod theme;
pub mod views;

pub use snapshot::{Screen, Snapshot, Status, StatusTone};
pub use theme::{Appearance, Theme, ThemeExt};
pub use views::DebrisApp;

/// Re-exports so the binary does not have to know GPUI's module layout.
pub mod prelude {
    pub use crate::components::button::ButtonVariant;
    pub use crate::icons::{Icon, icon};
    pub use crate::snapshot::{Screen, Snapshot, Status, StatusTone};
    pub use crate::theme::{Appearance, Theme, ThemeExt, radius, space, text};
    pub use crate::views::{DebrisApp, root};
}

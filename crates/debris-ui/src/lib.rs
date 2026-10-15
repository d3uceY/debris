//! The GPUI surface for debris.
//!
//! This crate knows about [`debris_core`] and GPUI, and nothing else. It does not
//! spawn a process, read a file, or know what PowerShell is. Behaviour arrives
//! through the [`DebrisApp`] trait, which the binary implements — which is what lets
//! the whole interface be laid out and reasoned about without an operating system
//! underneath it.
//!
//! # Where things live
//!
//! * [`theme`] — every colour, radius, height, and opacity. Nothing else may hold a
//!   raw value.
//! * [`icons`] — the drawn icon set, and the one place that explains how GPUI tints
//!   an SVG.
//! * [`components`] — the seven shapes the interface is made of, plus the shell's
//!   chrome pieces.
//! * [`views`] — the shell itself ([`views::root`]) and the five surfaces.
//! * [`snapshot`] — the plain data a frame is drawn from.
//!
//! Read `DESIGN.md` at the repository root before changing a token. The neutral
//! ramp, the zone heights, and the veil ladder are the three things every screen is
//! measured against.

pub mod components;
pub mod icons;
pub mod snapshot;
pub mod theme;
pub mod views;

pub use snapshot::{CatalogEntry, Pending, Screen, Snapshot, Status, StatusTone};
pub use theme::{Appearance, Theme, ThemeExt};
pub use views::DebrisApp;

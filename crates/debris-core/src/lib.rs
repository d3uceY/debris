//! Domain model and pure logic for `debris`.
//!
//! This crate deliberately has **no IO**. It does not spawn a process, read a file it
//! was not handed, or know that GPUI exists. Everything in here is a value in, a value
//! out — which is what makes the interesting parts of the app (what counts as
//! system-critical, what the ledger does, whether a script is safe to offer, whether a
//! tweak is already applied) testable in microseconds instead of by running things.

pub mod bloat;
pub mod catalog;
pub mod error;
pub mod extension;
pub mod json;
pub mod model;
pub mod plan;
pub mod preset;
pub mod script;
pub mod settings;
pub mod staging;
pub mod tweak;

pub use bloat::{BloatApp, BloatCatalog};
pub use catalog::SystemCatalog;
pub use error::{DebrisError, Result};
pub use extension::{Extension, is_fetchable};
pub use model::AppxPackage;
pub use plan::{Caveat, RemovalPlan, RemovalStep};
pub use preset::Preset;
pub use script::{Category, Risk, ScriptMeta};
pub use settings::Settings;
pub use staging::{Counts, Filter, Staging};
pub use tweak::{Hive, Restart, Tweak, TweakEntry, TweakScan, TweakStatus, ValueKind};

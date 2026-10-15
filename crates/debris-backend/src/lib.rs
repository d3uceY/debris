//! The PowerShell boundary.
//!
//! Everything that touches the operating system lives here, and it does so through a
//! single seam: [`CommandRunner`]. That seam is the reason this crate can be tested
//! completely — against recorded invocations, without starting a process, and
//! therefore without any chance of a test changing the machine it runs on.
//!
//! The three implementations are deliberate:
//!
//! - [`ProcessRunner`] actually spawns. Used by the app.
//! - [`DryRunRunner`] and [`ScriptedRunner`] are test doubles: they record the
//!   invocations and replay canned output, so a test never spawns anything.

pub mod discovery;
pub mod extensions;
pub mod fetch;
pub mod registry;
pub mod removal;
pub mod runner;
pub mod scripts;

pub use discovery::{LIST_PACKAGES, parse_packages};
pub use fetch::{checksum_matches, fetch, sha256};
pub use registry::{TweakOutcome, apply as apply_tweaks, reset as reset_tweaks, scan as scan_tweaks};
pub use removal::{RemovalOutcome, execute as execute_removal, uninstall_invocation};
pub use runner::{CommandRunner, DryRunRunner, Invocation, ProcessRunner, RunOutput, ScriptedRunner};
pub use scripts::{Environment, environment, run_path, run_script, script_invocation};

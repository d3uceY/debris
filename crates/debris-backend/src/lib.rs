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
//! - [`DryRunRunner`] records what it would have run and returns success. Used when
//!   the user is previewing.
//! - [`ScriptedRunner`] replays canned output. Used by tests.

pub mod discovery;
pub mod removal;
pub mod runner;
pub mod scripts;

pub use discovery::{LIST_PACKAGES, parse_packages};
pub use removal::{RemovalOutcome, execute as execute_removal, uninstall_invocation};
pub use runner::{CommandRunner, DryRunRunner, Invocation, ProcessRunner, RunOutput, ScriptedRunner};
pub use scripts::{is_elevated, run_script, script_invocation};

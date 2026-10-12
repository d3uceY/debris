use std::fmt;
use std::path::PathBuf;

/// Everything that can go wrong while reading the world.
///
/// There is no `#[from]` machinery here on purpose: this enum is small, the
/// messages are user-facing, and hand-writing `Display` keeps the crate
/// dependency-free and the wording under our control.
#[derive(Debug)]
pub enum DebrisError {
    /// A file could not be read.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// A script's `###` header is missing a required field.
    ScriptHeader { path: PathBuf, detail: String },
    /// No script library was found where one was expected.
    LibraryMissing { path: PathBuf },
    /// A process could not be started at all.
    Process {
        program: String,
        source: std::io::Error,
    },
    /// A process started but reported failure.
    Failed { program: String, detail: String },
}

impl fmt::Display for DebrisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DebrisError::Io { path, source } => {
                write!(f, "could not read {}: {source}", path.display())
            }
            DebrisError::ScriptHeader { path, detail } => {
                write!(f, "{} has an invalid script header: {detail}", path.display())
            }
            DebrisError::LibraryMissing { path } => {
                write!(f, "no script library at {}", path.display())
            }
            DebrisError::Process { program, source } => {
                write!(f, "could not start {program}: {source}")
            }
            DebrisError::Failed { program, detail } => {
                write!(f, "{program} failed: {detail}")
            }
        }
    }
}

impl std::error::Error for DebrisError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DebrisError::Io { source, .. } => Some(source),
            DebrisError::Process { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, DebrisError>;

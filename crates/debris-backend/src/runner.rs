use debris_core::{DebrisError, Result};
use std::path::Path;
use std::sync::Mutex;

/// A single process invocation, as data.
///
/// Keeping this a value rather than a `std::process::Command` is what makes the
/// whole backend testable: a test can assert on exactly what *would* run without
/// anything running.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    pub program: String,
    pub args: Vec<String>,
}

impl Invocation {
    /// Runs a PowerShell snippet. `-NonInteractive` matters: a script that tries to
    /// prompt should fail rather than hang a window forever.
    pub fn powershell(command: &str) -> Self {
        Self {
            program: "powershell.exe".to_string(),
            args: vec![
                "-NoProfile".to_string(),
                "-NonInteractive".to_string(),
                "-ExecutionPolicy".to_string(),
                // Process-scoped only. It is not written to the machine policy, and
                // some default configurations refuse to run any script without it.
                "Bypass".to_string(),
                "-Command".to_string(),
                command.to_string(),
            ],
        }
    }

    /// Runs a script from the library.
    ///
    /// `-ExecutionPolicy Bypass` is scoped to this one process; nothing about the
    /// machine's policy is modified.
    pub fn script(path: &Path, dry_run: bool) -> Self {
        let mut args = vec![
            "-NoProfile".to_string(),
            "-NonInteractive".to_string(),
            "-ExecutionPolicy".to_string(),
            "Bypass".to_string(),
            "-File".to_string(),
            path.display().to_string(),
        ];
        if dry_run {
            args.push("-DryRun".to_string());
        }
        Self {
            program: "powershell.exe".to_string(),
            args,
        }
    }

    /// The one-line form used in logs and error messages.
    pub fn display(&self) -> String {
        let mut out = self.program.clone();
        for arg in &self.args {
            out.push(' ');
            if arg.contains(' ') {
                out.push('"');
                out.push_str(arg);
                out.push('"');
            } else {
                out.push_str(arg);
            }
        }
        out
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunOutput {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

impl RunOutput {
    pub fn ok(status: i32, stdout: impl Into<String>) -> Self {
        Self {
            status,
            stdout: stdout.into(),
            stderr: String::new(),
        }
    }

    pub fn succeeded(&self) -> bool {
        self.status == 0
    }

    /// PowerShell writes a lot of its real errors to stdout rather than stderr, so
    /// callers want both when they are explaining a failure.
    pub fn combined(&self) -> String {
        let mut out = self.stdout.trim().to_string();
        let err = self.stderr.trim();
        if !err.is_empty() {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(err);
        }
        out
    }
}

/// The seam. Every OS-touching operation in this crate goes through it.
pub trait CommandRunner: Send + Sync {
    fn run(&self, invocation: &Invocation) -> Result<RunOutput>;
}

/// Actually spawns the process.
#[derive(Debug, Default)]
pub struct ProcessRunner;

impl CommandRunner for ProcessRunner {
    fn run(&self, invocation: &Invocation) -> Result<RunOutput> {
        use std::process::{Command, Stdio};

        let mut command = Command::new(&invocation.program);
        command
            .args(&invocation.args)
            // A script must never be able to block on a prompt. With no stdin and
            // -NonInteractive, it fails instead of hanging.
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Stop a console window flashing up in front of the user's app.
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let output = command.output().map_err(|source| DebrisError::Process {
            program: invocation.program.clone(),
            source,
        })?;

        Ok(RunOutput {
            // A process killed by a signal has no exit code; -1 is a clearer answer
            // than pretending it succeeded.
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// Records invocations and reports success without running anything.
///
/// A test double, like [`ScriptedRunner`]: the live preview is the `-DryRun` flag
/// handed to a script, not a runner. This exists so a test can assert on exactly
/// what would have run without spawning anything.
#[derive(Debug, Default)]
pub struct DryRunRunner {
    log: Mutex<Vec<Invocation>>,
}

impl DryRunRunner {
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything that would have run, in order.
    pub fn log(&self) -> Vec<Invocation> {
        self.log.lock().map(|log| log.clone()).unwrap_or_default()
    }

    pub fn clear(&self) {
        if let Ok(mut log) = self.log.lock() {
            log.clear();
        }
    }
}

impl CommandRunner for DryRunRunner {
    fn run(&self, invocation: &Invocation) -> Result<RunOutput> {
        if let Ok(mut log) = self.log.lock() {
            log.push(invocation.clone());
        }
        Ok(RunOutput {
            status: 0,
            stdout: format!("dry run: {}", invocation.display()),
            stderr: String::new(),
        })
    }
}

/// Replays canned responses, in order, and records what it was asked to do.
///
/// When the script runs out it returns a failure rather than silently succeeding,
/// so a test that under-specifies its expectations fails loudly.
#[derive(Debug, Default)]
pub struct ScriptedRunner {
    responses: Mutex<std::collections::VecDeque<RunOutput>>,
    log: Mutex<Vec<Invocation>>,
}

impl ScriptedRunner {
    pub fn new(responses: Vec<RunOutput>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            log: Mutex::new(Vec::new()),
        }
    }

    pub fn log(&self) -> Vec<Invocation> {
        self.log.lock().map(|log| log.clone()).unwrap_or_default()
    }
}

impl CommandRunner for ScriptedRunner {
    fn run(&self, invocation: &Invocation) -> Result<RunOutput> {
        if let Ok(mut log) = self.log.lock() {
            log.push(invocation.clone());
        }
        self.responses
            .lock()
            .ok()
            .and_then(|mut queue| queue.pop_front())
            .ok_or_else(|| DebrisError::Failed {
                program: invocation.program.clone(),
                detail: "no scripted response left — the test under-specified its expectations"
                    .to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_powershell_invocation_is_non_interactive() {
        let invocation = Invocation::powershell("Get-Date");
        assert_eq!(invocation.program, "powershell.exe");
        assert!(invocation.args.contains(&"-NoProfile".to_string()));
        assert!(invocation.args.contains(&"-NonInteractive".to_string()));
        assert_eq!(invocation.args.last().unwrap(), "Get-Date");
    }

    #[test]
    fn a_script_invocation_passes_dry_run_only_when_asked() {
        let path = Path::new("scripts/apps/remove-onedrive.ps1");
        let dry = Invocation::script(path, true);
        assert!(dry.args.contains(&"-DryRun".to_string()));

        let live = Invocation::script(path, false);
        assert!(!live.args.contains(&"-DryRun".to_string()));
        assert!(live.args.contains(&"-File".to_string()));
    }

    #[test]
    fn display_quotes_arguments_containing_spaces() {
        let invocation = Invocation::script(Path::new("C:/Program Files/x.ps1"), true);
        let shown = invocation.display();
        assert!(shown.contains("\"C:/Program Files/x.ps1\""));
    }

    #[test]
    fn the_dry_run_runner_records_instead_of_spawning() {
        let runner = DryRunRunner::new();
        let invocation = Invocation::powershell("Remove-Everything");
        let output = runner.run(&invocation).unwrap();
        assert!(output.succeeded());
        assert_eq!(runner.log(), vec![invocation]);
        runner.clear();
        assert!(runner.log().is_empty());
    }

    #[test]
    fn the_scripted_runner_replays_and_then_fails_loudly() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "first")]);
        let invocation = Invocation::powershell("x");
        assert_eq!(runner.run(&invocation).unwrap().stdout, "first");
        let err = runner.run(&invocation).unwrap_err();
        assert!(matches!(err, DebrisError::Failed { .. }));
        assert_eq!(runner.log().len(), 2);
    }

    #[test]
    fn combined_reports_both_streams() {
        let output = RunOutput {
            status: 1,
            stdout: "out".into(),
            stderr: "err".into(),
        };
        assert_eq!(output.combined(), "out\nerr");
        assert!(!output.succeeded());
    }
}

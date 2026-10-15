use crate::runner::{CommandRunner, Invocation, RunOutput};
use debris_core::{DebrisError, Result, ScriptMeta};
use std::path::Path;

/// The environment probe: are we elevated, and which Windows is this.
///
/// Deliberately a *query* rather than an attempt to elevate. debris never opens a
/// UAC prompt on its own initiative; it tells the user what a script needs and lets
/// them relaunch elevated.
///
/// Both answers come from one process because both are wanted at the same moment and
/// PowerShell costs a fifth of a second to start.
pub const ENVIRONMENT: &str = "$b=0;try{$b=[Environment]::OSVersion.Version.Build}catch{};'{0}|{1}' -f ([bool](([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))), $b";

/// What the probe found. `build` is `None` when the machine would not say.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Environment {
    pub elevated: bool,
    pub build: Option<u32>,
}

impl Environment {
    /// Windows 11 starts at build 22000. Unknown builds count as 11, because the
    /// catalogue's 11-only rules are the ones a current machine wants, and hiding
    /// them on a machine that would not answer is the worse failure.
    pub fn is_windows_11(&self) -> bool {
        self.build.is_none_or(|build| build >= 22000)
    }
}

pub fn environment(runner: &dyn CommandRunner) -> Environment {
    let output = runner.run(&Invocation::powershell(ENVIRONMENT));
    let text = output.map(|output| output.stdout).unwrap_or_default();
    let (elevated, build) = text.trim().split_once('|').unwrap_or(("", ""));
    Environment {
        elevated: elevated.trim().eq_ignore_ascii_case("true"),
        // Zero is what the probe leaves behind when the property threw.
        build: build.trim().parse::<u32>().ok().filter(|build| *build > 0),
    }
}

/// How a script is invoked.
pub fn script_invocation(meta: &ScriptMeta) -> Invocation {
    Invocation::script(&meta.path)
}

/// Runs a PowerShell file by path.
///
/// Extensions are downloaded rather than shipped, so they have no [`ScriptMeta`] and
/// no declared risk to check against. The confirmation that guards them happens in
/// the interface, and the path is built by the app from the URL's own filename.
pub fn run_path(path: &Path, runner: &dyn CommandRunner) -> Result<RunOutput> {
    runner.run(&Invocation::script(path))
}

/// Runs a script now. The interface asks for confirmation before it gets here.
///
/// A script the library has refused is refused here too, so the refusal cannot be
/// bypassed by a caller that skips the interface.
pub fn run_script(meta: &ScriptMeta, runner: &dyn CommandRunner) -> Result<RunOutput> {
    if !meta.is_runnable() {
        return Err(DebrisError::Failed {
            program: meta.id.clone(),
            detail: meta
                .blocked_reason()
                .unwrap_or("this script is not runnable")
                .to_string(),
        });
    }
    runner.run(&script_invocation(meta))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{DryRunRunner, ScriptedRunner};
    use debris_core::ScriptMeta;

    fn script(header: &str) -> ScriptMeta {
        ScriptMeta::parse(Path::new("scripts/apps/demo.ps1"), header).unwrap()
    }

    const REVERSIBLE: &str = "### title: Demo\n### category: apps\n### risk: reversible\n### dry-run: yes\n";
    const UNSAFE: &str = "### title: Bad\n### category: apps\n### risk: destructive\n### dry-run: no\n";

    #[test]
    fn a_script_is_invoked_as_a_file() {
        let meta = script(REVERSIBLE);
        let invocation = script_invocation(&meta);
        let file_at = invocation.args.iter().position(|arg| arg == "-File").unwrap();
        assert!(invocation.args[file_at + 1].ends_with("demo.ps1"));
        assert!(!invocation.args.contains(&"-DryRun".to_string()));
    }

    #[test]
    fn running_a_refused_script_is_an_error() {
        let meta = script(UNSAFE);
        let runner = DryRunRunner::new();
        let err = run_script(&meta, &runner).unwrap_err();
        assert!(matches!(err, DebrisError::Failed { .. }));
        assert!(
            runner.log().is_empty(),
            "a refused script must not even be invoked"
        );
    }

    #[test]
    fn a_safe_script_runs() {
        let meta = script(REVERSIBLE);
        let runner = DryRunRunner::new();
        assert!(run_script(&meta, &runner).unwrap().succeeded());
        assert_eq!(runner.log().len(), 1);
    }

    #[test]
    fn the_probe_reads_elevation_and_the_build_from_one_line() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "True|26100\r\n")]);
        let probe = environment(&runner);
        assert!(probe.elevated);
        assert_eq!(probe.build, Some(26100));
        assert!(probe.is_windows_11());
        assert_eq!(runner.log().len(), 1, "one process for both answers");
    }

    #[test]
    fn a_ten_build_is_not_windows_eleven() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "False|19045\r\n")]);
        let probe = environment(&runner);
        assert!(!probe.elevated);
        assert!(!probe.is_windows_11());
    }

    #[test]
    fn a_probe_that_fails_reports_standard_user_and_an_unknown_build() {
        let runner = ScriptedRunner::new(vec![RunOutput {
            status: 1,
            stdout: "nonsense".into(),
            stderr: String::new(),
        }]);
        let probe = environment(&runner);
        assert!(!probe.elevated);
        assert_eq!(probe.build, None);
        assert!(probe.is_windows_11(), "an unknown build keeps the newer rules");
    }

    #[test]
    fn a_zero_build_reads_as_unknown() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "False|0\r\n")]);
        assert_eq!(environment(&runner).build, None);
    }
}

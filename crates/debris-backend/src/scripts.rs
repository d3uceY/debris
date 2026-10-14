use crate::runner::{CommandRunner, Invocation, RunOutput};
use debris_core::{DebrisError, Result, ScriptMeta};

/// The elevation probe.
///
/// Deliberately a *query* rather than an attempt to elevate. debris never opens a
/// UAC prompt on its own initiative; it tells the user what a script needs and lets
/// them relaunch elevated.
pub const IS_ELEVATED: &str = "[bool](([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))";

pub fn is_elevated(runner: &dyn CommandRunner) -> bool {
    runner
        .run(&Invocation::powershell(IS_ELEVATED))
        .map(|output| output.stdout.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

/// How a script is invoked.
pub fn script_invocation(meta: &ScriptMeta) -> Invocation {
    Invocation::script(&meta.path)
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
    use std::path::Path;

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
    fn elevation_reads_true_from_powershell() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "True\r\n")]);
        assert!(is_elevated(&runner));
    }

    #[test]
    fn elevation_reads_false_from_powershell() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "False\r\n")]);
        assert!(!is_elevated(&runner));
    }

    #[test]
    fn elevation_assumes_no_when_the_probe_fails() {
        let runner = ScriptedRunner::new(vec![RunOutput {
            status: 1,
            stdout: "nonsense".into(),
            stderr: String::new(),
        }]);
        assert!(!is_elevated(&runner));
    }
}

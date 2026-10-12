use crate::runner::{CommandRunner, Invocation};
use debris_core::RemovalPlan;

/// The uninstall command for one package.
///
/// Keyed on `-Package <PackageFullName>` so the operation is exact. Staging by
/// name is a convenience for the user; acting on a name would be a pattern match
/// that could take more than they asked for.
pub fn uninstall_invocation(full_name: &str) -> Invocation {
    // PowerShell escapes a single quote inside a single-quoted string by doubling it.
    // A package full name never legitimately contains one, but an unescaped quote
    // would be command injection, so it is not left to chance.
    let escaped = full_name.replace('\'', "''");
    Invocation::powershell(&format!(
        "Get-AppxPackage -Package '{escaped}' | Remove-AppxPackage -ErrorAction Stop"
    ))
}

/// What happened to one package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemovalOutcome {
    pub name: String,
    pub full_name: String,
    pub removed: bool,
    pub detail: String,
}

impl RemovalOutcome {
    /// The line the status bar shows.
    pub fn summary(&self) -> String {
        if self.removed {
            format!("Removed {}", self.name)
        } else {
            format!("Could not remove {}: {}", self.name, self.detail)
        }
    }
}

/// Runs every step of the plan, in order.
///
/// A failure does not abort the rest. The user staged several packages and needs to
/// learn about all of them; stopping at the first error would hide the others and
/// leave the ledger lying about what is installed.
pub fn execute(plan: &RemovalPlan, runner: &dyn CommandRunner) -> Vec<RemovalOutcome> {
    plan.steps()
        .iter()
        .map(|step| match runner.run(&uninstall_invocation(&step.full_name)) {
            Ok(output) if output.succeeded() => RemovalOutcome {
                name: step.name.clone(),
                full_name: step.full_name.clone(),
                removed: true,
                detail: String::new(),
            },
            Ok(output) => RemovalOutcome {
                name: step.name.clone(),
                full_name: step.full_name.clone(),
                removed: false,
                detail: first_line(&output.combined()),
            },
            Err(error) => RemovalOutcome {
                name: step.name.clone(),
                full_name: step.full_name.clone(),
                removed: false,
                detail: error.to_string(),
            },
        })
        .collect()
}

/// PowerShell failures are verbose; the status line gets the first meaningful line.
fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("no output")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{DryRunRunner, RunOutput, ScriptedRunner};
    use debris_core::{AppxPackage, RemovalPlan, SystemCatalog};

    fn plan(names: &[&str]) -> RemovalPlan {
        let catalog = SystemCatalog::default();
        let packages: Vec<AppxPackage> = names
            .iter()
            .map(|name| {
                let mut package = AppxPackage::new(*name, format!("{name}_1.0.0.0_x64__abc"));
                catalog.classify(&mut package);
                package
            })
            .collect();
        RemovalPlan::from_packages(&packages)
    }

    #[test]
    fn the_uninstall_is_keyed_on_the_full_package_name() {
        let invocation = uninstall_invocation("Microsoft.BingNews_1.0.0.0_x64__abc");
        assert!(invocation.args.last().unwrap().contains(
            "Get-AppxPackage -Package 'Microsoft.BingNews_1.0.0.0_x64__abc' | Remove-AppxPackage"
        ));
    }

    #[test]
    fn a_quote_in_a_package_name_is_escaped_rather_than_injected() {
        let invocation = uninstall_invocation("Evil'; Remove-Item C:\\ -Recurse; '");
        let command = invocation.args.last().unwrap();
        assert!(command.contains("''"), "the quote should be doubled: {command}");
        assert!(!command.contains("Evil'; Remove-Item"), "injection should be neutralised");
    }

    #[test]
    fn a_successful_step_is_reported_as_removed() {
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "")]);
        let outcomes = execute(&plan(&["Microsoft.BingNews"]), &runner);
        assert_eq!(outcomes.len(), 1);
        assert!(outcomes[0].removed);
        assert_eq!(outcomes[0].summary(), "Removed Microsoft.BingNews");
    }

    #[test]
    fn a_failing_step_does_not_stop_the_others() {
        let runner = ScriptedRunner::new(vec![
            RunOutput {
                status: 1,
                stdout: "Remove-AppxPackage : Access is denied.".into(),
                stderr: String::new(),
            },
            RunOutput::ok(0, ""),
        ]);
        let outcomes = execute(&plan(&["A.First", "B.Second"]), &runner);
        assert_eq!(outcomes.len(), 2);
        assert!(!outcomes[0].removed);
        assert!(outcomes[1].removed);
        assert!(outcomes[0].summary().contains("Access is denied."));
    }

    #[test]
    fn an_empty_plan_runs_nothing() {
        let runner = DryRunRunner::new();
        assert!(execute(&plan(&[]), &runner).is_empty());
        assert!(runner.log().is_empty());
    }

    #[test]
    fn a_dry_run_records_the_commands_without_running_them() {
        let runner = DryRunRunner::new();
        let outcomes = execute(&plan(&["A.One", "B.Two"]), &runner);
        assert!(outcomes.iter().all(|outcome| outcome.removed));
        assert_eq!(runner.log().len(), 2);
        assert!(runner.log()[0].display().contains("Get-AppxPackage -Package 'A.One_1.0.0.0_x64__abc'"));
    }

    #[test]
    fn first_line_falls_back_when_there_is_no_output() {
        assert_eq!(first_line("   \n  "), "no output");
        assert_eq!(first_line("\n  real error  \n"), "real error");
    }
}

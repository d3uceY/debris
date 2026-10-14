//! The safety model, asserted against the real `scripts/` directory.
//!
//! These tests are the reason the README's three promises are facts rather than
//! intentions. **Nothing here executes a script.** The one test that touches
//! PowerShell hands it to the *parser* and never to the engine, so it cannot change
//! the machine it runs on.

use debris_backend::runner::{CommandRunner, Invocation, ProcessRunner, RunOutput};
use debris_backend::{is_elevated, script_invocation};
use debris_core::script::library;
use debris_core::{Risk, ScriptMeta};
use std::path::{Path, PathBuf};

fn scripts_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts")
}

fn library_or_panic() -> Vec<ScriptMeta> {
    match library(scripts_root()) {
        Ok(scripts) => scripts,
        Err(error) => panic!("the shipped script library did not load: {error}"),
    }
}

// ---------------------------------------------------------------------------
// 1. Every script describes itself.
// ---------------------------------------------------------------------------

#[test]
fn every_script_declares_a_complete_header() {
    let scripts = library_or_panic();
    assert!(
        scripts.len() >= 5,
        "expected a populated library, found {}",
        scripts.len()
    );
    for script in &scripts {
        assert!(!script.title.is_empty(), "{} has no title", script.id);
        assert!(!script.summary.is_empty(), "{} has no summary", script.id);
    }
}

#[test]
fn categories_match_the_directory_a_script_lives_in() {
    for script in library_or_panic() {
        let parent = script
            .path
            .parent()
            .and_then(|dir| dir.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        assert_eq!(
            parent,
            script.category.directory(),
            "{} declares category `{}` but sits in `{parent}/`",
            script.id,
            script.category.directory()
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Destructive scripts can always be previewed.
// ---------------------------------------------------------------------------

#[test]
fn a_destructive_script_always_supports_a_dry_run() {
    for script in library_or_panic() {
        if script.risk == Risk::Destructive {
            assert!(
                script.supports_dry_run,
                "{} is destructive but cannot be dry-run, so it would not load",
                script.id
            );
        }
    }
}

#[test]
fn every_script_in_the_library_is_runnable() {
    for script in library_or_panic() {
        assert!(
            script.is_runnable(),
            "{} is blocked: {:?}",
            script.id,
            script.blocked_reason()
        );
    }
}

#[test]
fn the_defensive_script_is_present_and_labeled_destructive() {
    let scripts = library_or_panic();
    let defender = scripts
        .iter()
        .find(|script| script.id.contains("disable-windows-defender"))
        .expect("the Defender script should be in the library");
    assert_eq!(
        defender.risk,
        Risk::Destructive,
        "disabling Defender must never be labelled reversible"
    );
    assert!(
        defender.summary_text().contains("NOT RECOMMENDED"),
        "the summary should say so plainly"
    );
}

// ---------------------------------------------------------------------------
// 3. Nothing mutates outside a dry-run-aware helper.
// ---------------------------------------------------------------------------

/// Native cmdlets that change the machine. Every one of these must appear inside an
/// `-Action { }` block (i.e. indented) or arrive via a `*-Debris*` helper, which
/// respects the dry-run flag by construction.
const MUTATING: [&str; 10] = [
    "Remove-Item",
    "Remove-AppxPackage",
    "Remove-NetFirewallRule",
    "Set-ItemProperty",
    "Set-Service",
    "Set-Content",
    "Stop-Service",
    "Unregister-ScheduledTask",
    "Add-AppxPackage",
    "New-NetFirewallRule",
];

#[test]
fn no_script_mutates_anything_at_the_top_level() {
    for script in library_or_panic() {
        let text = std::fs::read_to_string(&script.path).expect("script should be readable");
        for (number, line) in text.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            // Only a top-level statement (column zero) is at risk of running
            // unconditionally; anything indented sits inside a guarded block.
            if line.len() != trimmed.len() {
                continue;
            }
            // The `*-Debris*` helpers are the sanctioned route.
            if trimmed.contains("Debris") {
                continue;
            }
            for token in MUTATING {
                assert!(
                    !trimmed.contains(token),
                    "{}:{} mutates at the top level (`{token}`) — route it through a \
                     dry-run-aware helper instead:\n    {trimmed}",
                    script.id,
                    number + 1
                );
            }
        }
    }
}

#[test]
fn every_script_uses_the_shared_dry_run_plumbing() {
    for script in library_or_panic() {
        let text = std::fs::read_to_string(&script.path).expect("script should be readable");
        assert!(
            text.contains("[switch]$DryRun"),
            "{} does not accept -DryRun",
            script.id
        );
        assert!(
            text.contains("Debris.Common.psm1"),
            "{} does not import the shared module",
            script.id
        );
        assert!(
            text.contains("Use-DebrisMode"),
            "{} never calls Use-DebrisMode, so -DryRun would do nothing",
            script.id
        );
        assert!(
            text.contains("Complete-DebrisRun"),
            "{} does not report a final summary",
            script.id
        );
    }
}

#[test]
fn a_script_that_needs_admin_says_so_in_its_header() {
    for script in library_or_panic() {
        let text = std::fs::read_to_string(&script.path).expect("script should be readable");
        let asserts_admin = text.contains("Assert-DebrisAdmin")
            || text.contains("Assert-DebrisProcessElevated");
        assert_eq!(
            asserts_admin, script.requires_admin,
            "{} declares requires-admin={} but {} assert elevation",
            script.id,
            script.requires_admin,
            if asserts_admin { "does" } else { "does not" }
        );
    }
}

// ---------------------------------------------------------------------------
// 4. The scripts are syntactically valid PowerShell.
// ---------------------------------------------------------------------------

/// Hands a file to PowerShell's **parser** and returns any syntax errors.
///
/// `None` means PowerShell could not be reached at all, and the caller should skip
/// rather than fail. Parsing a file does not run it.
fn parse_errors(path: &Path) -> Option<Vec<String>> {
    let quoted = path.display().to_string().replace('\'', "''");
    let probe = format!(
        "$errors = $null; \
         [System.Management.Automation.Language.Parser]::ParseFile('{quoted}', [ref]$null, [ref]$errors) | Out-Null; \
         if ($errors) {{ $errors | ForEach-Object {{ $_.Message }} }} else {{ 'DEBRIS-PARSE-OK' }}"
    );
    let output: RunOutput = ProcessRunner.run(&Invocation::powershell(&probe)).ok()?;
    let text = output.combined();
    if text.contains("DEBRIS-PARSE-OK") {
        return Some(Vec::new());
    }
    Some(
        text.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
            .collect(),
    )
}

#[test]
fn every_script_parses_under_powershells_own_parser() {
    let mut checked = 0;
    for script in library_or_panic() {
        let Some(errors) = parse_errors(&script.path) else {
            eprintln!("skipping: PowerShell is not available on this machine");
            return;
        };
        assert!(
            errors.is_empty(),
            "{} does not parse: {errors:#?}",
            script.path.display()
        );
        checked += 1;
    }
    assert!(checked >= 5, "expected to check the whole library, checked {checked}");
}

#[test]
fn the_shared_module_parses() {
    let module = scripts_root().join("lib/Debris.Common.psm1");
    let Some(errors) = parse_errors(&module) else {
        eprintln!("skipping: PowerShell is not available on this machine");
        return;
    };
    assert!(errors.is_empty(), "{} does not parse: {errors:#?}", module.display());
}

// ---------------------------------------------------------------------------
// 5. The runner contract holds.
// ---------------------------------------------------------------------------

#[test]
fn powershell_is_always_launched_non_interactively() {
    for script in library_or_panic() {
        let invocation = script_invocation(&script);
        assert_eq!(invocation.program, "powershell.exe");
        assert!(invocation.args.contains(&"-NoProfile".to_string()));
        assert!(
            invocation.args.contains(&"-NonInteractive".to_string()),
            "{} could hang on a prompt without -NonInteractive",
            script.id
        );
    }
}

#[test]
fn the_elevation_probe_asks_rather_than_tries() {
    // A probe that attempted to elevate would open a UAC prompt as soon as the app
    // started. Assert it only ever reads a role membership.
    assert!(debris_backend::scripts::IS_ELEVATED.contains("IsInRole"));
    assert!(!debris_backend::scripts::IS_ELEVATED.contains("RunAs"));
    assert!(!debris_backend::scripts::IS_ELEVATED.contains("Start-Process"));
}

#[test]
fn elevation_is_reported_as_false_when_the_probe_fails() {
    use debris_backend::runner::ScriptedRunner;
    let runner = ScriptedRunner::new(vec![RunOutput {
        status: 1,
        stdout: String::new(),
        stderr: "denied".into(),
    }]);
    assert!(!is_elevated(&runner));
}

#[test]
fn the_dry_run_runner_never_spawns_a_process() {
    use debris_backend::runner::DryRunRunner;
    let runner = DryRunRunner::new();
    let invocation = Invocation::powershell("Remove-Item C:\\ -Recurse -Force");
    assert!(runner.run(&invocation).unwrap().succeeded());
    let log = runner.log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0], invocation);
}

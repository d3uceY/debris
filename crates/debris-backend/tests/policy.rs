//! The safety model, asserted against the real `scripts/` directory.
//!
//! These tests are the reason the README's three promises are facts rather than
//! intentions. **Nothing here executes a script.** The one test that touches
//! PowerShell hands it to the *parser* and never to the engine, so it cannot change
//! the machine it runs on.

use debris_backend::runner::{CommandRunner, Invocation, ProcessRunner, RunOutput};
use debris_backend::{environment, script_invocation};
use debris_core::script::library;
use debris_core::{BloatCatalog, Extension, Risk, ScriptMeta, SystemCatalog, Tweak};
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
    run_parse_probe(&probe)
}

/// The same, for a script that exists only as a string.
fn parse_text_errors(text: &str) -> Option<Vec<String>> {
    let quoted = text.replace('\'', "''");
    let probe = format!(
        "$errors = $null; \
         [System.Management.Automation.Language.Parser]::ParseInput('{quoted}', [ref]$null, [ref]$errors) | Out-Null; \
         if ($errors) {{ $errors | ForEach-Object {{ $_.Message }} }} else {{ 'DEBRIS-PARSE-OK' }}"
    );
    run_parse_probe(&probe)
}

/// Runs a probe that reports syntax errors one per line, or `DEBRIS-PARSE-OK`.
fn run_parse_probe(probe: &str) -> Option<Vec<String>> {
    let output: RunOutput = ProcessRunner.run(&Invocation::powershell(probe)).ok()?;
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

/// The registry scripts are *generated*, so no file on disk can vouch for them. This
/// is the only thing that reads them the way PowerShell will.
#[test]
fn the_generated_registry_scripts_parse() {
    use debris_backend::registry::{apply_script, reset_script, scan_script};

    let tweaks = Tweak::parse_library(&asset("fixers.json")).expect("fixers.json should parse");
    let scripts = [
        ("scan", scan_script(&tweaks)),
        ("apply", apply_script(&tweaks)),
        ("reset", reset_script(&tweaks)),
    ];
    for (name, script) in scripts {
        let Some(errors) = parse_text_errors(&script) else {
            eprintln!("skipping: PowerShell is not available on this machine");
            return;
        };
        assert!(errors.is_empty(), "the {name} script does not parse: {errors:#?}");
    }
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
    assert!(debris_backend::scripts::ENVIRONMENT.contains("IsInRole"));
    assert!(!debris_backend::scripts::ENVIRONMENT.contains("RunAs"));
    assert!(!debris_backend::scripts::ENVIRONMENT.contains("Start-Process"));
}

#[test]
fn elevation_is_reported_as_false_when_the_probe_fails() {
    use debris_backend::runner::ScriptedRunner;
    let runner = ScriptedRunner::new(vec![RunOutput {
        status: 1,
        stdout: String::new(),
        stderr: "denied".into(),
    }]);
    assert!(!environment(&runner).elevated);
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

// ---------------------------------------------------------------------------
// 6. The shipped catalogues.
//
// These read the real files under `assets/data`. A catalogue that does not load is
// not a runtime error the interface can recover from — it is a thinner app nobody
// asked for — so it fails the build instead.
// ---------------------------------------------------------------------------

fn asset(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../assets/data")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn every_shipped_fixer_is_a_rule_the_app_can_apply_and_undo() {
    let tweaks = Tweak::parse_library(&asset("fixers.json")).expect("fixers.json should parse");
    assert!(
        tweaks.len() >= 40,
        "expected a real catalogue, found {}",
        tweaks.len()
    );
    for tweak in &tweaks {
        assert!(!tweak.name.is_empty());
        assert!(!tweak.description.is_empty(), "{} has no description", tweak.name);
        assert!(
            !tweak.entries.is_empty(),
            "{} writes nothing, so it can never do anything",
            tweak.name
        );
        for entry in &tweak.entries {
            assert!(!entry.key.is_empty(), "{} has an empty key", tweak.name);
            // A rule with no default can be applied but never taken back, so the
            // surface would offer a restore that silently skips it.
            assert!(
                entry.is_restorable(),
                "{}: `{}` has no Windows default to restore",
                tweak.name,
                entry.display_name()
            );
        }
    }
}

#[test]
fn no_two_shipped_fixers_share_a_name() {
    let tweaks = Tweak::parse_library(&asset("fixers.json")).unwrap();
    let mut names: Vec<String> = tweaks
        .iter()
        .map(|tweak| tweak.name.to_ascii_lowercase())
        .collect();
    let total = names.len();
    names.sort();
    names.dedup();
    assert_eq!(total, names.len(), "a duplicate name would shadow a rule");
}

#[test]
fn every_shipped_fixer_declares_a_category() {
    let tweaks = Tweak::parse_library(&asset("fixers.json")).unwrap();
    for tweak in &tweaks {
        assert_ne!(
            tweak.category_label(),
            "Other",
            "{} falls into no category",
            tweak.name
        );
    }
}

#[test]
fn a_machine_needing_elevation_is_always_declared_as_such() {
    let tweaks = Tweak::parse_library(&asset("fixers.json")).unwrap();
    let inferring = tweaks
        .iter()
        .filter(|tweak| {
            tweak.entries.iter().any(|entry| {
                entry.hive.needs_admin()
                    || entry.key.to_ascii_lowercase().contains("\\policies\\")
            })
        })
        .count();
    assert!(
        inferring > 0,
        "the catalogue should contain rules that need admin, or the inference is untested"
    );
    for tweak in &tweaks {
        let should = tweak.entries.iter().any(|entry| {
            entry.hive.needs_admin() || entry.key.to_ascii_lowercase().contains("\\policies\\")
        });
        assert_eq!(
            tweak.requires_admin, should,
            "{} disagrees with its own entries about elevation",
            tweak.name
        );
    }
}

#[test]
fn every_shipped_bloatware_entry_is_matchable() {
    let catalog =
        BloatCatalog::parse(&asset("bloatware.json")).expect("bloatware.json should parse");
    assert!(
        catalog.len() >= 30,
        "expected a real catalogue, found {}",
        catalog.len()
    );
    for app in catalog.apps() {
        assert!(!app.package.is_empty(), "{} has no package name", app.name);
        assert!(!app.description.is_empty(), "{} has no description", app.name);
        assert!(
            catalog.find(&app.package).is_some(),
            "{} cannot be found by its own package name",
            app.name
        );
    }
}

#[test]
fn the_recommended_bloatware_set_is_a_real_subset() {
    let catalog = BloatCatalog::parse(&asset("bloatware.json")).unwrap();
    let recommended = catalog.recommended();
    assert!(
        !recommended.is_empty() && recommended.len() < catalog.len(),
        "the recommended set should be a proper subset, found {} of {}",
        recommended.len(),
        catalog.len()
    );
}

#[test]
fn the_system_catalogue_covers_every_package_the_bloatware_one_recommends() {
    // A recommended removal that Windows needs is the one combination this project
    // must never ship: the fixer would tell someone to break their own machine.
    let system = SystemCatalog::parse(&asset("system-apps.txt"));
    let catalog = BloatCatalog::parse(&asset("bloatware.json")).unwrap();
    for package in catalog.recommended() {
        assert!(
            !system.contains(&package),
            "{package} is recommended for removal and marked system-critical"
        );
    }
}

// ---------------------------------------------------------------------------
// 7. The extension template.
//
// `examples/extensions/` is documentation. A template that does not load teaches the
// wrong shape, and one whose URLs point at nothing teaches a shape nobody can copy —
// so it is checked the way the shipped catalogues are, and its scripts are handed to
// PowerShell's parser like every other script in this repository.
// ---------------------------------------------------------------------------

#[test]
fn the_extension_template_is_a_catalogue_that_loads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/extensions");
    let text = std::fs::read_to_string(root.join("catalog.json"))
        .unwrap_or_else(|error| panic!("the example catalogue is missing: {error}"));
    let entries = Extension::parse_catalog(&text).expect("the example catalogue should parse");

    for name in ["Export network configuration", "Export recent security events"] {
        assert!(
            entries.iter().any(|entry| entry.name == name),
            "{name} is in the template but the loader drops it"
        );
    }
    assert!(
        entries.iter().any(|entry| entry.requires_admin),
        "the template should show the admin flag, or nobody learns it exists"
    );

    for entry in &entries {
        assert!(entry.is_fetchable(), "{} is not an http(s) URL", entry.name);
        assert!(
            !entry.description.is_empty(),
            "{} has no description to show before a run",
            entry.name
        );
        // The URL's tail is the installed file name, which makes the template's URLs
        // a claim about this folder. Assert the claim.
        let name = entry
            .file_name_of()
            .unwrap_or_else(|| panic!("{} does not name a .ps1 file", entry.name));
        let file = root.join(&name);
        assert!(
            file.is_file(),
            "{} is listed as {name}, which is not in the template folder",
            entry.name
        );
        let Some(errors) = parse_errors(&file) else {
            eprintln!("skipping: PowerShell is not available on this machine");
            return;
        };
        assert!(
            errors.is_empty(),
            "{} does not parse: {errors:#?}",
            file.display()
        );
    }
}

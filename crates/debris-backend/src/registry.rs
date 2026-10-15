//! Reading and writing registry values, through PowerShell and nothing else.
//!
//! Every function here builds an [`Invocation`] or drives one through a
//! [`CommandRunner`]. So the whole surface is testable against a runner that records
//! instead of running, which matters more here than anywhere else in the crate: a
//! test that reads the registry is a test that passes or fails depending on whose
//! machine it is on.
//!
//! # One process per batch
//!
//! PowerShell costs about a fifth of a second to start. Sixty-one tweaks read one at
//! a time would be twelve seconds of nothing. So a batch is a single invocation that
//! walks every entry and prints one line per entry, tagged with its position.
//!
//! # Why the write script wraps each tweak
//!
//! A tweak that needs `HKLM` fails on a standard user's session, and a batch of
//! twenty should not lose the other nineteen to it. Each tweak gets its own
//! `try`/`catch`, its own result line, and the exit code reports whether any of them
//! fell over.

use crate::runner::{CommandRunner, Invocation, RunOutput};
use debris_core::{DebrisError, Result, Tweak, TweakEntry, TweakScan, ValueKind};
use std::collections::BTreeMap;

/// What the read script prints when a value is not there. A marker rather than an
/// empty line, because "absent" and "present and empty" are different answers — the
/// classic context menu tweak depends on that difference.
pub const UNSET: &str = "__DEBRIS_UNSET__";

/// The hive table, the number parser, and the hex reader. Shared by every script.
///
/// Nothing here writes. That is deliberate: what a *scan* can do to the machine is
/// decided by what its script contains, so the read path gets a prelude that has no
/// key-creating call in it at all.
const PROLOGUE: &str = concat!(
    "$ErrorActionPreference='Stop'\n",
    "$H=@{HKCU=[Microsoft.Win32.Registry]::CurrentUser;",
    "HKLM=[Microsoft.Win32.Registry]::LocalMachine;",
    "HKCR=[Microsoft.Win32.Registry]::ClassesRoot;",
    "HKU=[Microsoft.Win32.Registry]::Users}\n",
    // Registry numbers may be written in either base.
    "function N([string]$v){if($v.StartsWith('0x')){[uint64]::Parse($v.Substring(2),",
    "[Globalization.NumberStyles]::HexNumber)}else{[uint64]::Parse($v)}}\n",
    // A run of hex with whatever separators the data used, as a byte array.
    // The leading comma stops PowerShell from unrolling it into the caller.
    "function H([string]$v){$c=$v -replace '[^0-9A-Fa-f]','';",
    "if($c.Length % 2 -ne 0){throw 'a binary value needs an even number of hex digits'};",
    "$b=[byte[]]::new($c.Length/2);",
    "for($i=0;$i -lt $b.Length;$i++){$b[$i]=[Convert]::ToByte($c.Substring($i*2,2),16)};return ,$b}\n",
);

/// The write-only half of the prelude: opening a key for writing, the one call that
/// knows how each value type is stored, and the two calls that un-write one.
///
/// `D` and `X` are here for every batch, including the ones that never call them.
/// They exist because a tweak's default is sometimes the *absence* of a value rather
/// than a value, so a reset has to be able to remove what it cannot set. Working out
/// whether a given batch needs them costs more to reason about than an unused function
/// definition costs to ship.
const WRITE_PRELUDE: &str = concat!(
    "function K([string]$hive,[string]$key){$r=$H[$hive];",
    "if($null -eq $r){throw \"unknown hive $hive\"};",
    "$k=$r.CreateSubKey($key);",
    "if($null -eq $k){throw \"cannot open $hive\\$key\"};$k}\n",
    "function D([string]$hive,[string]$key){$r=$H[$hive];",
    "if($null -ne $r){$r.DeleteSubKeyTree($key,$false)}}\n",
    "function X([string]$hive,[string]$key,[string]$name){$r=$H[$hive];",
    "if($null -ne $r){$k=$r.OpenSubKey($key,$true);",
    "if($null -ne $k){$n=if($name -eq '(Default)'){''}else{$name};",
    "$k.DeleteValue($n,$false);$k.Close()}}}\n",
    "function S([string]$hive,[string]$key,[string]$name,[string]$kind,[string]$value){\n",
    "  $k=K $hive $key\n",
    "  $n=if($name -eq '(Default)'){''}else{$name}\n",
    "  switch($kind){\n",
    "    'dword'{$u=[uint32](N $value);",
    "$k.SetValue($n,[BitConverter]::ToInt32([BitConverter]::GetBytes($u),0),",
    "[Microsoft.Win32.RegistryValueKind]::DWord)}\n",
    "    'qword'{$q=[uint64](N $value);",
    "$k.SetValue($n,[BitConverter]::ToInt64([BitConverter]::GetBytes($q),0),",
    "[Microsoft.Win32.RegistryValueKind]::QWord)}\n",
    "    'binary'{$k.SetValue($n,[byte[]](H $value),",
    "[Microsoft.Win32.RegistryValueKind]::Binary)}\n",
    "    'expandstring'{$k.SetValue($n,$value,",
    "[Microsoft.Win32.RegistryValueKind]::ExpandString)}\n",
    "    default{$k.SetValue($n,$value,[Microsoft.Win32.RegistryValueKind]::String)}\n",
    "  }\n",
    "  $k.Close()\n",
    "}\n",
);

/// A single-quoted PowerShell literal. An unescaped quote would be command
/// injection, and the catalogue is not necessarily ours.
fn ps(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// `Name` as the registry sees it: the default value has no name.
const NAME_EXPR: &str = "$n=if($name -eq '(Default)'){''}else{$name}";

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// The script that reads every entry of every tweak.
///
/// Emits `<index><tab><value>` per entry, with [`UNSET`] where nothing is stored.
pub fn scan_script(tweaks: &[Tweak]) -> String {
    let mut work = String::new();
    for tweak in tweaks {
        for entry in &tweak.entries {
            work.push_str(&format!(
                "{}\n",
                ps(&format!(
                    "{}|{}|{}",
                    entry.hive.short(),
                    entry.key,
                    entry.display_name()
                ))
            ));
        }
    }

    format!(
        "{PROLOGUE}\
$W=@(\n{work})\n\
$i=0\n\
foreach($w in $W){{\n\
  $p=$w -split '\\|',3\n\
  $v='{UNSET}'\n\
  $r=$H[$p[0]]\n\
  if($null -ne $r){{\n\
    $k=$r.OpenSubKey($p[1])\n\
    if($null -ne $k){{\n\
      $name=$p[2]\n\
      {NAME_EXPR}\n\
      $x=$k.GetValue($n,$null)\n\
      if($null -ne $x){{\n\
        if($x -is [byte[]]){{$v=([BitConverter]::ToString($x)) -replace '-',''}}\n\
        else{{$v=([string]$x) -replace \"[\\r\\n]\",' '}}\n\
      }}\n\
      $k.Close()\n\
    }}\n\
  }}\n\
  Write-Output (\"$i`t$v\")\n\
  $i++\n\
}}\n"
    )
}

pub fn scan_invocation(tweaks: &[Tweak]) -> Invocation {
    Invocation::powershell(&scan_script(tweaks))
}

/// Reads every entry, then decides each tweak's status.
pub fn scan(runner: &dyn CommandRunner, tweaks: &[Tweak]) -> Result<Vec<TweakScan>> {
    if tweaks.is_empty() {
        return Ok(Vec::new());
    }
    let output = runner.run(&scan_invocation(tweaks))?;
    if !output.succeeded() {
        return Err(DebrisError::Failed {
            program: "powershell".to_string(),
            detail: first_line(&output.combined()),
        });
    }
    let values = parse_values(&output.stdout);
    let mut index = 0usize;
    Ok(tweaks
        .iter()
        .map(|tweak| {
            tweak.scan(|_| {
                let value = values.get(&index).cloned().flatten();
                index += 1;
                value
            })
        })
        .collect())
}

/// `<index><tab><value>` per line, with [`UNSET`] meaning "nothing stored".
///
/// Only the first tab splits: a value is allowed to contain one.
fn parse_values(stdout: &str) -> BTreeMap<usize, Option<String>> {
    numbered_lines(stdout)
        .map(|(index, value)| {
            let value = value.trim_end_matches('\r');
            (
                index,
                if value == UNSET {
                    None
                } else {
                    Some(value.to_string())
                },
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// Which value a write puts there.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    /// The tweak's own recommendation.
    Recommended,
    /// The value Windows shipped with, or its absence.
    Default,
}

/// A `.reg` document describing what the registry holds right now, for a batch.
///
/// One extra read, before anything is written. Best effort by contract: a value that
/// cannot be read is simply not in the export, because refusing to run the batch the
/// user asked for is a worse outcome than a backup that is one value short.
pub fn backup(runner: &dyn CommandRunner, tweaks: &[Tweak]) -> Result<String> {
    if tweaks.is_empty() {
        return Ok(String::new());
    }
    let output = runner.run(&scan_invocation(tweaks))?;
    if !output.succeeded() {
        return Err(DebrisError::Failed {
            program: "powershell".to_string(),
            detail: first_line(&output.combined()),
        });
    }
    Ok(backup_text(tweaks, parse_values(&output.stdout)))
}

/// Renders the `.reg` body from already-read values.
///
/// Split out from [`backup`] so the export can be checked without a runner.
fn backup_text(tweaks: &[Tweak], values: BTreeMap<usize, Option<String>>) -> String {
    let mut index = 0usize;
    Tweak::backup_text(tweaks, |_| {
        let value = values.get(&index).cloned().flatten();
        index += 1;
        value
    })
}

/// What happened to one tweak in a batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TweakOutcome {
    pub name: String,
    pub ok: bool,
    /// PowerShell's message when it failed, empty otherwise.
    pub detail: String,
}

impl TweakOutcome {
    /// The line the status bar shows.
    pub fn summary(&self) -> String {
        if self.ok {
            format!("Applied {}", self.name)
        } else {
            format!("{} failed: {}", self.name, self.detail)
        }
    }
}

/// The script that writes a batch, reporting one line per tweak.
fn write_script(tweaks: &[Tweak], target: Target) -> String {
    let mut body = String::from(PROLOGUE);
    body.push_str(WRITE_PRELUDE);
    body.push_str("$ok=$true\n");

    for (index, tweak) in tweaks.iter().enumerate() {
        let edits = write_edits(tweak, target);
        if edits.is_empty() {
            // Nothing to write is not a failure; it is a tweak with no way back.
            body.push_str(&format!(
                "Write-Output (\"{index}`t\" + 'nothing to write')\n"
            ));
            continue;
        }
        body.push_str(&format!(
            "try{{\n{edits}  Write-Output (\"{index}`tOK\")\n}}\
catch{{$ok=$false;Write-Output (\"{index}`t\" + $_.Exception.Message)}}\n"
        ));
    }

    body.push_str("if(-not $ok){exit 1}\n");
    body
}

/// The `S`/delete calls for one tweak, indented into its `try` block.
fn write_edits(tweak: &Tweak, target: Target) -> String {
    let mut edits = String::new();
    for entry in &tweak.entries {
        match target {
            Target::Recommended => {
                edits.push_str(&format!("  {}\n", set_call(entry, &entry.recommended)));
            }
            Target::Default => {
                let default = entry.default.trim();
                if default.is_empty() {
                    continue;
                }
                match default.to_ascii_lowercase().as_str() {
                    // The default is the absence of the key, not a value in it.
                    "<delete>" => edits.push_str(&format!(
                        "  D {} {}\n",
                        ps(entry.hive.short()),
                        ps(&entry.key)
                    )),
                    "<deletevalue>" => edits.push_str(&format!(
                        "  X {} {} {}\n",
                        ps(entry.hive.short()),
                        ps(&entry.key),
                        ps(entry.display_name())
                    )),
                    _ => edits.push_str(&format!("  {}\n", set_call(entry, &entry.default))),
                }
            }
        }
    }
    edits
}

fn set_call(entry: &TweakEntry, value: &str) -> String {
    format!(
        "S {} {} {} {} {}",
        ps(entry.hive.short()),
        ps(&entry.key),
        ps(entry.display_name()),
        ps(kind_name(entry.kind)),
        ps(value)
    )
}

fn kind_name(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::Dword => "dword",
        ValueKind::Qword => "qword",
        ValueKind::String => "string",
        ValueKind::ExpandString => "expandstring",
        ValueKind::Binary => "binary",
    }
}

/// Applies a batch, one process for the lot.
pub fn apply(runner: &dyn CommandRunner, tweaks: &[Tweak]) -> Vec<TweakOutcome> {
    run_batch(runner, tweaks, Target::Recommended)
}

/// Puts a batch back to the values Windows ships with.
pub fn reset(runner: &dyn CommandRunner, tweaks: &[Tweak]) -> Vec<TweakOutcome> {
    run_batch(runner, tweaks, Target::Default)
}

/// The text [`apply`] runs. Exposed so a test can hand it to PowerShell's parser.
pub fn apply_script(tweaks: &[Tweak]) -> String {
    write_script(tweaks, Target::Recommended)
}

/// The text [`reset`] runs.
pub fn reset_script(tweaks: &[Tweak]) -> String {
    write_script(tweaks, Target::Default)
}

fn run_batch(runner: &dyn CommandRunner, tweaks: &[Tweak], target: Target) -> Vec<TweakOutcome> {
    if tweaks.is_empty() {
        return Vec::new();
    }
    let invocation = Invocation::powershell(&write_script(tweaks, target));
    let output = runner.run(&invocation);
    let (results, transport) = match output {
        Ok(output) => (parse_outcomes(&output.stdout), transport_error(&output)),
        Err(error) => (BTreeMap::new(), Some(error.to_string())),
    };

    tweaks
        .iter()
        .enumerate()
        .map(|(index, tweak)| match results.get(&index) {
            Some(Ok(())) => TweakOutcome {
                name: tweak.name.clone(),
                ok: true,
                detail: String::new(),
            },
            Some(Err(detail)) => TweakOutcome {
                name: tweak.name.clone(),
                ok: false,
                detail: detail.clone(),
            },
            None => TweakOutcome {
                name: tweak.name.clone(),
                ok: false,
                detail: transport.clone().unwrap_or_else(|| "no result".to_string()),
            },
        })
        .collect()
}

/// `<index><tab>OK`, or `<index><tab><message>`.
fn parse_outcomes(stdout: &str) -> BTreeMap<usize, std::result::Result<(), String>> {
    numbered_lines(stdout)
        .map(|(index, result)| {
            let result = result.trim_end_matches('\r');
            if result == "OK" {
                (index, Ok(()))
            } else {
                (index, Err(result.to_string()))
            }
        })
        .collect()
}

/// The one line shape both readers above are handed: a position, a tab, the rest.
///
/// A line without the tab, or with something that is not a number in front of it, is
/// skipped rather than guessed at. PowerShell is allowed to emit noise and the batch
/// it belongs to must not become a parse error over a stray banner line.
fn numbered_lines(stdout: &str) -> impl Iterator<Item = (usize, &str)> {
    stdout.lines().filter_map(|line| {
        let (index, rest) = line.split_once('\t')?;
        Some((index.trim().parse::<usize>().ok()?, rest))
    })
}

/// PowerShell writes most of its real errors to stdout, so a non-zero exit with no
/// usable result line still has something to say.
fn transport_error(output: &RunOutput) -> Option<String> {
    (!output.succeeded()).then(|| first_line(&output.combined()))
}

/// PowerShell failures are verbose; one line is enough to act on.
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
    use debris_core::{Hive, Restart};

    fn entry(key: &str, name: &str, kind: ValueKind, recommended: &str, default: &str) -> TweakEntry {
        TweakEntry {
            hive: Hive::CurrentUser,
            key: key.to_string(),
            value_name: name.to_string(),
            kind,
            recommended: recommended.to_string(),
            default: default.to_string(),
        }
    }

    fn tweak(name: &str, entries: Vec<TweakEntry>) -> Tweak {
        Tweak {
            name: name.to_string(),
            category: "Test".to_string(),
            description: String::new(),
            entries,
            restart: Restart::None,
            requires_admin: false,
            default_selected: false,
            os: None,
        }
    }

    fn script_of(invocation: &Invocation) -> &str {
        invocation.args.last().expect("a command argument")
    }

    #[test]
    fn a_backup_exports_the_values_that_were_read() {
        let tweaks = vec![tweak(
            "d",
            vec![
                entry("a", "first", ValueKind::Dword, "0", "1"),
                entry("b", "second", ValueKind::Dword, "0", "1"),
            ],
        )];
        let values = parse_values(&format!("0\t3\n1\t{UNSET}\n"));
        let text = backup_text(&tweaks, values);
        assert!(text.starts_with("Windows Registry Editor Version 5.00"));
        assert!(text.contains("[HKEY_CURRENT_USER\\a]"));
        assert!(text.contains("\"first\"=dword:00000003"));
        assert!(!text.contains("second"), "an unset value has nothing to export");
    }

    #[test]
    fn a_backup_of_nothing_never_starts_powershell() {
        let runner = DryRunRunner::new();
        assert_eq!(backup(&runner, &[]).unwrap(), "");
        assert!(runner.log().is_empty());
    }

    #[test]
    fn a_read_script_walks_every_entry_in_order() {
        let tweaks = vec![
            tweak(
                "one",
                vec![
                    entry("Software\\A", "First", ValueKind::Dword, "0", "1"),
                    entry("Software\\B", "Second", ValueKind::String, "x", "y"),
                ],
            ),
            tweak("two", vec![entry("Software\\C", "(Default)", ValueKind::String, "", "<delete>")]),
        ];
        let invocation = scan_invocation(&tweaks);
        let script = script_of(&invocation);
        assert!(script.contains("'HKCU|Software\\A|First'"));
        assert!(script.contains("'HKCU|Software\\B|Second'"));
        assert!(script.contains("'HKCU|Software\\C|(Default)'"));
        // Reading must never create a key it was only asked to look at.
        assert!(!script.contains("CreateSubKey"));
        assert!(script.contains("OpenSubKey"));
    }

    #[test]
    fn the_read_script_quotes_the_sentinel() {
        // Written unquoted, PowerShell reads `__DEBRIS_UNSET__` as a command name and
        // the whole scan dies with "The term ... is not recognized". This test exists
        // because that is exactly what happened the first time it ran.
        let tweaks = vec![tweak("x", vec![entry("a", "v", ValueKind::Dword, "1", "0")])];
        let script = scan_script(&tweaks);
        assert!(
            script.contains(&format!("$v='{UNSET}'")),
            "the sentinel has to be a string literal: {script}"
        );
    }

    #[test]
    fn a_scan_maps_values_back_onto_their_tweaks() {
        let tweaks = vec![
            tweak(
                "two part",
                vec![
                    entry("a", "first", ValueKind::Dword, "0", "1"),
                    entry("b", "second", ValueKind::Dword, "1", "0"),
                ],
            ),
            tweak("single", vec![entry("c", "third", ValueKind::Dword, "5", "5")]),
        ];
        let runner = ScriptedRunner::new(vec![RunOutput::ok(
            0,
            "0\t0\n1\t1\n2\t5\n",
        )]);
        let scans = scan(&runner, &tweaks).unwrap();
        assert_eq!(scans.len(), 2);
        assert_eq!(scans[0].status, debris_core::TweakStatus::Applied);
        assert_eq!(scans[0].current.as_deref(), Some("0"));
        assert_eq!(scans[1].status, debris_core::TweakStatus::Applied);
        assert_eq!(scans[1].current.as_deref(), Some("5"));
    }

    #[test]
    fn an_unset_marker_reads_as_not_applied() {
        let tweaks = vec![tweak("x", vec![entry("a", "v", ValueKind::Dword, "1", "0")])];
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, format!("0\t{UNSET}\n"))]);
        let scans = scan(&runner, &tweaks).unwrap();
        assert_eq!(scans[0].status, debris_core::TweakStatus::NotApplied);
        assert_eq!(scans[0].current, None);
    }

    #[test]
    fn a_value_containing_a_tab_survives_the_round_trip() {
        let values = parse_values("0\ta\tb\n1\tplain\n");
        assert_eq!(values.get(&0).cloned().flatten().as_deref(), Some("a\tb"));
        assert_eq!(values.get(&1).cloned().flatten().as_deref(), Some("plain"));
    }

    #[test]
    fn a_failed_scan_is_an_error_rather_than_a_guess() {
        let tweaks = vec![tweak("x", vec![entry("a", "v", ValueKind::Dword, "1", "0")])];
        let runner = ScriptedRunner::new(vec![RunOutput {
            status: 1,
            stdout: String::new(),
            stderr: "Access is denied.".to_string(),
        }]);
        let error = scan(&runner, &tweaks).unwrap_err();
        assert!(error.to_string().contains("Access is denied"));
    }

    #[test]
    fn the_write_script_writes_the_recommended_value_with_the_right_kind() {
        let tweaks = vec![
            tweak("d", vec![entry("a", "v", ValueKind::Dword, "0", "1")]),
            tweak("q", vec![entry("b", "v", ValueKind::Qword, "7", "0")]),
            tweak("s", vec![entry("c", "v", ValueKind::String, "hello", "bye")]),
            tweak("x", vec![entry("d", "v", ValueKind::ExpandString, "%TEMP%", "")]),
            tweak("b", vec![entry("e", "v", ValueKind::Binary, "0A0B", "")]),
        ];
        let script = write_script(&tweaks, Target::Recommended).to_string();
        assert!(script.contains("S 'HKCU' 'a' 'v' 'dword' '0'"));
        assert!(script.contains("S 'HKCU' 'b' 'v' 'qword' '7'"));
        assert!(script.contains("S 'HKCU' 'c' 'v' 'string' 'hello'"));
        assert!(script.contains("S 'HKCU' 'd' 'v' 'expandstring' '%TEMP%'"));
        assert!(script.contains("S 'HKCU' 'e' 'v' 'binary' '0A0B'"));
        // Each tweak reports its own result.
        for index in 0..5 {
            assert!(script.contains(&format!("Write-Output (\"{index}`tOK\")")));
        }
    }

    #[test]
    fn a_default_of_delete_removes_the_key_and_deletevalue_only_the_value() {
        let tweaks = vec![tweak(
            "back to nothing",
            vec![
                entry("a", "(Default)", ValueKind::String, "", "<delete>"),
                entry("b", "v", ValueKind::Dword, "1", "<deletevalue>"),
            ],
        )];
        let script = write_script(&tweaks, Target::Default);
        assert!(script.contains("D 'HKCU' 'a'"), "{script}");
        assert!(script.contains("X 'HKCU' 'b' 'v'"), "{script}");
        // Both helpers have to be defined when they are used.
        assert!(script.contains("function D("));
        assert!(script.contains("function X("));
    }

    #[test]
    fn an_entry_with_no_default_is_left_alone_by_a_restore() {
        let tweaks = vec![tweak(
            "half restorable",
            vec![
                entry("a", "v", ValueKind::Dword, "1", "0"),
                entry("b", "v", ValueKind::Dword, "1", ""),
            ],
        )];
        let script = write_script(&tweaks, Target::Default);
        assert!(script.contains("S 'HKCU' 'a' 'v' 'dword' '0'"));
        assert!(!script.contains("'b'"), "no default means nothing to restore");
    }

    #[test]
    fn a_tweak_with_nothing_to_restore_reports_that_rather_than_failing() {
        let tweaks = vec![tweak("n", vec![entry("a", "v", ValueKind::Dword, "1", "")])];
        let script = write_script(&tweaks, Target::Default);
        assert!(script.contains("nothing to write"));
        assert!(!script.contains("try{"), "nothing to try means nothing to catch");
    }

    #[test]
    fn a_quote_in_a_path_cannot_break_out_of_the_command() {
        let tweaks = vec![tweak(
            "injection",
            vec![entry("a'; Remove-Item C:\\ -Recurse; '", "v", ValueKind::Dword, "0", "1")],
        )];
        let script = write_script(&tweaks, Target::Recommended);
        assert!(script.contains("''"), "the quote should be doubled: {script}");
        assert!(
            !script.contains("a'; Remove-Item"),
            "the injected command must not survive"
        );
    }

    #[test]
    fn a_batch_reports_success_and_failure_per_tweak() {
        let tweaks = vec![
            tweak("fine", vec![entry("a", "v", ValueKind::Dword, "0", "1")]),
            tweak("denied", vec![entry("b", "v", ValueKind::Dword, "0", "1")]),
        ];
        let runner = ScriptedRunner::new(vec![RunOutput::ok(0, "0\tOK\n1\tAccess is denied.\n")]);
        let outcomes = apply(&runner, &tweaks);
        assert!(outcomes[0].ok);
        assert_eq!(outcomes[0].summary(), "Applied fine");
        assert!(!outcomes[1].ok);
        assert!(outcomes[1].detail.contains("Access is denied"));
    }

    #[test]
    fn a_batch_that_never_reported_blames_the_transport_once() {
        let tweaks = vec![tweak("x", vec![entry("a", "v", ValueKind::Dword, "0", "1")])];
        let runner = ScriptedRunner::new(vec![RunOutput {
            status: 1,
            stdout: String::new(),
            stderr: "The term 'K' is not recognized.".to_string(),
        }]);
        let outcomes = apply(&runner, &tweaks);
        assert!(!outcomes[0].ok);
        assert!(outcomes[0].detail.contains("not recognized"));
    }

    #[test]
    fn an_empty_batch_never_starts_powershell() {
        let runner = DryRunRunner::new();
        assert!(apply(&runner, &[]).is_empty());
        assert!(scan(&runner, &[]).unwrap().is_empty());
        assert!(runner.log().is_empty());
    }

    #[test]
    fn a_dry_run_records_exactly_one_invocation_for_a_batch() {
        let tweaks = vec![
            tweak("a", vec![entry("a", "v", ValueKind::Dword, "0", "1")]),
            tweak("b", vec![entry("b", "v", ValueKind::Dword, "0", "1")]),
        ];
        let runner = DryRunRunner::new();
        apply(&runner, &tweaks);
        assert_eq!(runner.log().len(), 1, "one process for the whole batch");
        assert!(runner.log()[0].display().contains("powershell.exe -NoProfile"));
    }
}

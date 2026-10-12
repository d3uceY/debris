# The debris script library

PowerShell scripts that `debris` discovers and runs. The folder is a library, not a
build step: **nothing here runs on its own.** Each script is listed in the interface
with its own declared summary, and only ever runs when you press run.

## The header contract

Every script **must** open with a `###` header, because the header *is* the metadata —
there is no sidecar file to drift out of sync. The header ends at the first line that
is neither `###` nor blank, so a `###` comment further down the file is just a comment.

```powershell
### title: Remove OneDrive
### category: apps
### risk: reversible
### requires-admin: yes
### dry-run: yes
###
### Removes the OneDrive client and disables its sync policy.
### More prose lines are appended to the summary.
```

| Field | Required | Values |
|---|---|---|
| `title` | yes | Human title shown in the list |
| `category` | yes | `apps`, `privacy`, `shell`, `maintenance` |
| `risk` | yes | `read-only`, `reversible`, `destructive` |
| `requires-admin` | yes | `yes` / `no` |
| `dry-run` | yes | `yes` / `no` |
| `author` | no | Attribution |
| anything else | no | Treated as prose and appended to the summary |

A script with a missing or invalid field **fails to load**, and the failure names the
field. `crates/debris-core/src/script.rs` owns the parser; the tests there assert it.

## The safety rules

1. **`risk: destructive` requires `dry-run: yes`.** A script that declares itself
   destructive but cannot preview is refused by the library — it never even appears as
   runnable. This rule is enforced in code and asserted against this directory by
   `crates/debris-backend/tests/policy.rs`.
2. **`-DryRun` must be honoured for every mutation.** Do not check the flag yourself;
   route every change through `Invoke-DebrisStep`, `Remove-DebrisAppx`,
   `Disable-DebrisService`, or `Set-DebrisRegistryValue`. Those helpers return early in
   dry-run mode, which means a script cannot forget.
3. **Never self-elevate.** Call `Assert-DebrisAdmin -Reason '...'`. It throws a legible
   error instead of opening a UAC prompt behind the user's back.

## Writing a script

```powershell
### title: Something
### category: privacy
### risk: reversible
### requires-admin: yes
### dry-run: yes
###
### One or two sentences on what this changes and what it does not.

[CmdletBinding()]
param([switch]$DryRun)

Import-Module (Join-Path $PSScriptRoot '..\lib\Debris.Common.psm1') -Force
Use-DebrisMode -DryRun:$DryRun
Assert-DebrisAdmin -Reason 'change the thing'

Invoke-DebrisStep 'change the thing' -Action {
    # the actual mutation
}

Complete-DebrisRun
```

Run it yourself with a preview — this is the only safe way to test one:

```powershell
pwsh -NoProfile -File scripts/privacy/disable-unwanted-services.ps1 -DryRun
```

## Layout

```
lib/            shared module: logging, dry-run plumbing, elevation checks
apps/           install, remove, or restore preinstalled applications
privacy/        telemetry, background services, network rules
shell/          Explorer and Start menu
maintenance/    elevated system-level tasks
```

`lib/` is skipped by the loader — shared modules are not user-facing scripts.

## Testing without changing anything

The test suite parses every script with PowerShell's own parser and never executes one:

```sh
cargo test -p debris-backend --test policy
```

To check a script by hand without running it:

```powershell
[System.Management.Automation.Language.Parser]::ParseFile(
    (Resolve-Path 'scripts/privacy/block-telemetry.ps1'), [ref]$null, [ref]$errors)
$errors
```

## Attribution

`apps/remove-onedrive.ps1`, `apps/remove-default-apps.ps1`, and
`privacy/disable-unwanted-services.ps1` are adapted from
[Debloat-Windows-10](https://github.com/W4RH4WK/Debloat-Windows-10) (MIT). They have
been rewritten onto the dry-run helpers above and had their sharpest edges removed —
the OneDrive script no longer kills Explorer, and the app list no longer reaches for
take-ownership privileges.

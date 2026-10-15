# PRODUCT.md — debris

## What it is

A Windows debloat tool: it inventories the Appx/UWP packages installed for the current user and lets you stage a set for removal; it offers a catalogue of registry rules, one checkbox each, with the value Windows ships with recorded so every one can be undone; and it runs audited PowerShell maintenance scripts from a local library, with the ticked set confirmed before it runs.

## Who uses it

A single technical user, on their own Windows machine, deliberately. Not an installer's first-run wizard. They know what `Microsoft.BingNews` is and they want to see the exact list before anything happens. Session is short and focused: open, scan, stage, review, confirm, done. Ambient light: a desk, in daylight. That is why the surface is light.

## Jobs

1. "What is actually installed on this machine?" — see the real inventory, not a guess.
2. "Get rid of these specific things." — stage a set, review it, remove it.
3. "Don't let me break my machine." — know what is system-critical, what is reversible, and what will run before it runs.
4. "Do the same cleanup again later." — save and reload a named preset.
5. "Turn off the Windows annoyances." — tick the rules you want, see whether the machine already satisfies each one, and write them in a single pass.
6. "Undo it." — put a rule back to the value Windows ships with, from the catalogue's own default.
7. "Run one of the maintenance scripts." — read what it does, then run it, without surprises.
8. "Add my own script to the list." — point the app at a JSON catalogue and get its entries, without the app shipping anyone else's code.

## Surfaces

| Surface | Purpose |
|---|---|
| Packages | The inventory, the staging ledger, and the removal queue. The primary surface. |
| Fixers | The registry rule catalogue, grouped by category, one checkbox per rule. |
| Scripts | The PowerShell library, grouped by category, each with its declared summary and risk. |
| Extensions | Scripts from JSON catalogues the user adds by URL. |
| Presets | Named package sets: Fresh Start, and the user's own. |

## Platform

Windows only, by nature — the work is Appx package management, registry writes, and PowerShell execution. The UI is GPUI, which builds and runs on Windows via the `gpui_windows` backend.

## Constraints

- **Nothing destructive happens without an explicit, informed confirm.** The staging list and both tick sets are proposals, never actions.
- **Scripts declare their own risk.** The header names the category, the risk, and whether admin is needed, and the row shows it before the script runs.
- **A fixer names its way back.** Every rule carries the value Windows ships with, so restore is a fact about the catalogue rather than a promise. A rule whose default is the absence of a value says so, and restoring deletes it.
- Elevation is never silent. If a rule or a script needs admin, say so before it runs, not after it fails.
- **No network call the user did not ask for.** The app is entirely local until a catalogue URL is added, and then only that URL is read, once, when asked. Nothing is polled. There is no telemetry and no update check.
- Machine-local only: no accounts, no sync.

## Voice

Plain, declarative, specific. The interface states what will happen and what did. It does not congratulate, warn breathlessly, or use exclamation marks.

- Good: `12 of 118 packages staged for removal.`
- Good: `Requires administrator. Nothing has run yet.`
- Good: `31 not set`
- Bad: `You're about to supercharge your PC! 🚀`

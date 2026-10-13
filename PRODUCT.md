# PRODUCT.md — debris

## What it is

A Windows debloat tool: it inventories the Appx/UWP packages installed for the current user, lets you stage a set for removal, and removes them. It also runs audited PowerShell maintenance scripts from a local library, with every script previewed before it runs.

## Who uses it

A single technical user, on their own Windows machine, deliberately. Not an installer's first-run wizard. They know what `Microsoft.BingNews` is and they want to see the exact list before anything happens. Session is short and focused: open, scan, stage, review, confirm, done. Ambient light: a desk, in daylight. That is why the surface is light.

## Jobs

1. "What is actually installed on this machine?" — see the real inventory, not a guess.
2. "Get rid of these specific things." — stage a set, review it, remove it.
3. "Don't let me break my machine." — know what is system-critical, what is reversible, and what will run before it runs.
4. "Do the same cleanup again later." — save and reload a named preset.
5. "Run one of the maintenance scripts." — read what it does, then run it, without surprises.

## Surfaces

| Surface | Purpose |
|---|---|
| Packages | The inventory and the staging ledger. The primary surface. |
| Scripts | The PowerShell library, grouped by category, each with its declared summary and risk. |
| Presets | Named package sets: Fresh Start, and the user's own. |

## Platform

Windows only, by nature — the work is Appx package management and PowerShell execution. The UI is GPUI, which builds and runs on Windows via the `gpui_windows` backend.

## Constraints

- **Nothing destructive happens without an explicit, informed confirm.** The staging list is a proposal, never an action.
- **Dry-run is the default for scripts.** A script must declare its risk and support `-DryRun` to be listed as runnable.
- Elevation is never silent. If a script needs admin, say so before it runs, not after it fails.
- No network calls. No telemetry. No auto-update phone-home (the original shipped one; this does not).
- Machine-local only: no accounts, no sync.

## Voice

Plain, declarative, specific. The interface states what will happen and what did. It does not congratulate, warn breathlessly, or use exclamation marks.

- Good: `12 of 118 packages staged for removal.`
- Good: `Requires administrator. Nothing has run yet.`
- Bad: `You're about to supercharge your PC! 🚀`

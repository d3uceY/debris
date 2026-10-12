# debris

A Windows debloat tool in Rust. It inventories the Appx/UWP packages installed for
the current user, lets you stage a set for removal, and removes them. It also runs
audited PowerShell maintenance scripts from a local library, previewing each one
before it runs.

## Status

Early. The package inventory, staging ledger, script library, presets, and the full
interface are implemented and tested. Package removal executes the real PowerShell
command; scripts default to a dry run.

## Architecture

Four crates, one direction of dependency. The UI never touches PowerShell, and the
domain logic never touches the OS — which is what makes the whole thing testable
without running a single destructive thing.

```
crates/
├── debris-core/      domain model + all pure logic         (zero dependencies)
├── debris-backend/   PowerShell discovery, execution, and the script library
├── debris-ui/        GPUI: theme, icons, components, views (depends on core only)
└── debris-app/       the binary; wires backend and ui together
```

```
debris-app ──► debris-backend ──► debris-core
     └───────► debris-ui ───────► debris-core
```

- **`debris-core`** — `model`, `catalog` (what counts as system-critical), `staging`
  (the two-partition staging model the removal queue is a view of), `preset`, `script`
  (the `###` header contract), `plan` (turning a staging set into executable steps),
  `error`. No IO, no `std::process`,
  no GPUI. Everything here is unit-testable in microseconds.
- **`debris-backend`** — the PowerShell boundary, behind one trait. `CommandRunner`
  has a real implementation (spawns `powershell.exe`), a `DryRun` implementation,
  and a `Scripted` one for tests. Discovery asks PowerShell for **CSV** rather than
  marshalling objects, so parsing is plain, verifiable text handling.
- **`debris-ui`** — theme tokens, the SVG icon set, components, and views. Reads
  state through a small `Snapshot` type, so it can render any state without knowing
  where it came from.
- **`debris-app`** — asset source, window, and the controller that owns the state
  and moves work off the render thread.

### The safety model

Three rules, enforced by code and by tests rather than by convention:

1. **Nothing runs implicitly.** Removal and scripts are only ever triggered from the
   staging ledger or the script list, on an explicit action.
2. **Scripts declare themselves.** Every `.ps1` carries a `###` header naming its
   category, risk, whether it needs admin, and whether it supports `-DryRun`. The
   library refuses to mark a script runnable if it declares `risk=destructive`
   without `dry-run=yes`.
3. **Dry run is the default.** Scripts are invoked with `-DryRun` unless the user has
   explicitly turned it off for that run.

`crates/debris-backend/tests/policy.rs` asserts all three against the real
`scripts/` directory, including that every script parses under PowerShell's own
parser — parse-only, so validation never executes anything.

## Layout

```
crates/          the four Rust crates
scripts/         the PowerShell library, segmented by intent
  lib/           shared module (logging, elevation checks, dry-run plumbing)
  apps/          remove or restore preinstalled apps
  privacy/       telemetry, services
  shell/         Explorer and Start menu
  maintenance/   elevated system tasks
assets/
  icons/         the SVG icon set
  data/          system-app list, shipped presets
```

## Build

Requires Rust 1.85+ and Windows.

```sh
cargo run -p debris-app
```

The first build takes a few minutes — GPUI is a git dependency, so Cargo clones the
Zed repository and compiles on the order of 200 crates. After that it is around a
minute.

## Test

```sh
cargo test --workspace            # unit + policy tests
cargo test --release -p debris-backend --test perf -- --nocapture
```

The policy tests parse the real scripts with PowerShell's parser. **No script is
ever executed by the test suite**, and no test mutates the system.

## License

Apache-2.0. See [LICENSE](LICENSE). The interface this was inspired by is MIT-licensed;
see [CONTRIBUTING.md](CONTRIBUTING.md) for how third-party material in `references/` is treated.

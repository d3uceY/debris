# Contributing to debris

Thanks for taking a look. `debris` is a GPUI desktop app, so most of the workflow is ordinary Rust — plus a few GPUI-specific traps worth reading before your first build.

## Before you start

- Rust via [rustup](https://rustup.rs/), stable toolchain (1.99 or newer).
- GPUI runs on macOS, Linux, and Windows (Windows goes through the `gpui_windows` backend). Platform-specific system dependencies are listed in the `gpui` crate README.

## Getting it running

```sh
git clone <your-fork-url> debris
cd debris
cargo run
```

The first build takes a few minutes. `gpui` is a **git** dependency, so Cargo clones the Zed repository and compiles on the order of 200 crates. After that, rebuilds are around a minute.

## Project layout

| Path | What it is |
|---|---|
| `src/main.rs` | App entry point and the root view |
| `Cargo.toml` | Dependencies (both GPUI crates are pinned to one git revision) |
| `references/` | Vendored third-party reference material, not compiled |

## GPUI traps

These bite everybody once:

- **Start the app with `gpui_platform::application()`.** `gpui::Application::new()` no longer exists. Any example or blog post using it — or using `ViewContext`, or `cx.new_view(...)` — predates the current API and will not compile.
- **Both GPUI crates are git dependencies and must stay in sync.** `gpui` and `gpui_platform` have to resolve to the same revision, or you get confusing type mismatches.
- **Keep `edition` at 2024 or lower** in `Cargo.toml`. A newer value fails to parse before anything compiles, with an error that looks like a toolchain problem but is not.
- **The first-party examples are the best reference.** See `crates/gpui/examples/` in the Zed checkout under `~/.cargo/git/checkouts/`.

## Making a change

- Keep `cargo build` green, and run `cargo fmt` and `cargo clippy` before you push.
- GPUI has no CSS and no transforms on elements — motion is opacity, color, depth, size, and layout. If you are adding animation, plan around what the framework actually supports.
- **Look at the window.** Run the app and resize it small, medium, wide, and narrow. Fixed pixel widths that happen to work at one size are the most common defect in this codebase.

## Pull requests

- One logical change per PR. Small PRs get reviewed; sprawling ones sit.
- Describe how you verified it — the commands you ran and what you saw in the window. Screenshots help for anything visual.
- If you bump the GPUI revision, run the app and confirm the window still renders before opening the PR.

## License

Contributions are accepted under the **Apache License 2.0** (see [LICENSE](LICENSE)). By submitting a pull request you confirm you have the right to submit the work and agree that it may be distributed under those terms.

## Code of conduct

Participation in this project is covered by the [Code of Conduct](CODE_OF_CONDUCT.md).

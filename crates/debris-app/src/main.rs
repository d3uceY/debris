//! debris — a Windows debloat tool.
//!
//! Boot is deliberately short: register the asset source, install the theme global,
//! open one window, and let the view do the rest.
//!
//! A released build is launched by double-clicking it, so it is a `windows` subsystem
//! binary and Windows never gives it a console. Debug builds keep theirs, which is
//! where the `debris: ..` diagnostics go; `cfg(test)` keeps it too, or a release-mode
//! test run would swallow its own output.
#![cfg_attr(all(not(test), not(debug_assertions)), windows_subsystem = "windows")]

mod assets;
mod bundle;
mod root;

use crate::assets::Assets;
use crate::root::Root;
use debris_ui::prelude::*;
// `AppContext` — which provides `cx.new(..)` and `cx.set_global(..)` — arrives with
// GPUI's prelude.
use gpui::{App, Bounds, WindowBounds, WindowOptions, prelude::*, px, size};
use gpui_platform::application;

fn main() {
    application()
        // The icon set is embedded in the binary; see `assets` and `bundle`.
        .with_assets(Assets)
        .run(|cx: &mut App| {
            // Light is the default; see DESIGN.md for why the scene picks it.
            cx.set_global(Theme::light());

            let bounds = Bounds::centered(None, size(px(1320.0), px(860.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |_window, cx| cx.new(Root::new),
            )
            .expect("the window should open");

            cx.activate(true);
        });
}

//! debris — a Windows debloat tool.
//!
//! Boot is deliberately short: register the asset source, install the theme global,
//! open one window, and let the view do the rest.

mod assets;
mod paths;
mod root;

use crate::assets::Assets;
use crate::paths::Layout;
use crate::root::Root;
use debris_ui::prelude::*;
// `AppContext` — which provides `cx.new(..)` and `cx.set_global(..)` — arrives with
// GPUI's prelude.
use gpui::{App, Bounds, WindowBounds, WindowOptions, prelude::*, px, size};
use gpui_platform::application;

fn main() {
    let layout = Layout::discover();

    application()
        // The icon set is data on disk, resolved through this source.
        .with_assets(Assets::new(layout.icon_root()))
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

//! The Packages surface — the primary one.
//!
//! A table of everything installed, and the queue it feeds. The table's column
//! header is not decoration: it is what lets the publisher and version columns be
//! read at a glance rather than parsed per row.
//!
//! The surface owns exactly three controls (the filter is global and lives in the
//! title bar), and the order is deliberate: the switch that changes *what is in the
//! table*, then the count, then the one bulk action.

use crate::components::button::button;
use crate::components::panel::{panel_body, panel_header};
use crate::components::rows::package_columns;
use crate::components::{Tone, chip, empty_state, error_state, loading_bar, switch};
use crate::icons::Icon;
use crate::snapshot::{Screen, Snapshot};
use crate::theme::{Theme, space};
use crate::views::{DebrisApp, shared};
use gpui::{
    AnyElement, ClickEvent, Context, FocusHandle, MouseButton, MouseDownEvent, WeakEntity, div,
    prelude::*, px,
};

pub fn screen<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    root: &WeakEntity<T>,
    list_focus: &FocusHandle,
    cx: &mut Context<T>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .bg(theme.background)
        .child(panel_header(
            theme,
            Screen::Packages.title(),
            subtitle(snapshot),
            None,
        ))
        .child(toolbar::<T>(theme, snapshot, cx))
        .child(package_columns(theme))
        .when(snapshot.is_busy(), |el| {
            el.child(loading_bar(theme, "inventory-loading"))
        })
        .child(inventory::<T>(theme, snapshot, root, list_focus, cx))
        .into_any_element()
}

fn subtitle(snapshot: &Snapshot) -> String {
    let staged = snapshot.counts.staged;
    let base = format!(
        "{} installed · {} shown",
        snapshot.counts.installed,
        snapshot.inventory_len()
    );
    if staged == 0 {
        base
    } else {
        format!("{base} · {staged} staged")
    }
}

fn toolbar<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    let shown = snapshot.inventory_len();
    let empty = shown == 0;

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::SM))
        .h(px(crate::theme::metrics::TOOLBAR))
        .w_full()
        .flex_shrink_0()
        .px(px(space::MD))
        .bg(theme.band(crate::theme::veil::BAND_QUIET))
        .border_b_1()
        .border_color(theme.border)
        .child(switch(
            theme,
            "show-system",
            snapshot.show_system,
            "Show system components",
            cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                let next = !this.snapshot().show_system;
                this.set_show_system(next, cx);
            }),
        ))
        .child(div().flex_1())
        .child(chip(theme, format!("{shown} shown"), Tone::Quiet))
        .child(
            button(theme, "stage-all-visible")
                .label("Stage all shown")
                .icon(Icon::ArrowRight)
                .outline()
                .disabled(empty || snapshot.is_busy())
                .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.stage_all(cx)
                })),
        )
        .into_any_element()
}

/// The table. Its own focus handle, so arrow keys move a cursor through the rows
/// without stealing focus from the search field.
fn inventory<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    root: &WeakEntity<T>,
    list_focus: &FocusHandle,
    cx: &mut Context<T>,
) -> AnyElement {
    let mut column = panel_body()
        .track_focus(list_focus)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this: &mut T, _event: &MouseDownEvent, window, cx| {
                window.focus(this.list_focus(), cx);
            }),
        )
        .on_key_down(shared::list_key_handler(cx));

    // Four different nothings, four different sentences. "Empty" is only truthful
    // once we know *why* it is empty.
    let body: AnyElement = if let Some(error) = &snapshot.error {
        error_state(
            theme,
            "The inventory could not be read",
            &format!("{error}\nPowerShell could not reach the Appx store. Rescan to try again."),
        )
    } else if snapshot.inventory_is_empty() && !snapshot.scanned {
        empty_state(
            theme,
            Icon::Package,
            "No inventory yet",
            "Rescan asks Windows what is installed for this user.",
            Some(
                button(theme, "empty-rescan")
                    .label("Rescan")
                    .icon(Icon::Refresh)
                    .primary()
                    .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                        this.scan(cx)
                    })),
            ),
        )
    } else if snapshot.inventory_is_empty() && snapshot.filter_is_active() {
        empty_state(
            theme,
            Icon::Search,
            "Nothing matches that filter",
            "Clear the search field to see the whole inventory.",
            Some(
                button(theme, "empty-clear-filter")
                    .label("Clear the filter")
                    .icon(Icon::Close)
                    .outline()
                    .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                        this.set_query(String::new(), cx)
                    })),
            ),
        )
    } else if snapshot.inventory_is_empty() {
        empty_state(
            theme,
            Icon::ArrowRight,
            "Everything is in the queue",
            "Nothing has been removed yet.",
            None,
        )
    } else {
        shared::package_table::<T>(
            theme,
            "inventory",
            snapshot.inventory.clone(),
            snapshot.staged_names.clone(),
            false,
            Some(snapshot.cursor),
            root,
        )
    };

    column = column.child(body);
    column.into_any_element()
}

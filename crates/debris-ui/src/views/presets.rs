//! The Presets surface: named package sets.
//!
//! A preset is a file of package names, so the row is a name, a count, and the one
//! action. The action says what it does to the *queue* rather than what it does to
//! the machine, because loading a preset never removes anything.

use crate::components::button::button;
use crate::components::panel::{panel_body, panel_header};
use crate::components::rows;
use crate::components::{Tone, chip, empty_state};
use crate::icons::Icon;
use crate::snapshot::{Screen, Snapshot};
use crate::theme::{Theme, space, veil};
use crate::views::{DebrisApp, shared};
use debris_core::Preset;
use gpui::{AnyElement, ClickEvent, Context, div, prelude::*, px};

pub fn screen<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    let visible: Vec<&Preset> = snapshot
        .presets
        .iter()
        .filter(|preset| matches(preset, &snapshot.query))
        .collect();

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .bg(theme.background)
        .child(panel_header(
            theme,
            Screen::Presets.title(),
            subtitle(snapshot, visible.len()),
            None,
        ))
        .child(toolbar(theme, snapshot, visible.len()))
        .child(body::<T>(theme, snapshot, &visible, cx))
        .into_any_element()
}

fn subtitle(snapshot: &Snapshot, shown: usize) -> String {
    let total = snapshot.presets.len();
    let scope = if shown == total {
        shared::count(total, "saved set", "saved sets")
    } else {
        format!("{shown} of {total} shown")
    };
    format!(
        "{scope} · {} queued",
        shared::count(snapshot.staged.len(), "package", "packages")
    )
}

fn matches(preset: &Preset, query: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    query.is_empty() || preset.name.to_ascii_lowercase().contains(&query)
}

fn toolbar(theme: &Theme, snapshot: &Snapshot, shown: usize) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::SM))
        .h(px(crate::theme::metrics::TOOLBAR))
        .w_full()
        .flex_shrink_0()
        .px(px(space::MD))
        .bg(theme.band(veil::BAND_QUIET))
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_family(theme.font_ui.clone())
                .text_size(px(crate::theme::text::SMALL))
                .text_color(theme.muted_foreground)
                .child("Loading a preset replaces the removal queue. Nothing is removed until you confirm."),
        )
        .child(chip(theme, format!("{shown} shown"), Tone::Quiet))
        .child(chip(
            theme,
            format!("{} queued", snapshot.staged.len()),
            Tone::Neutral,
        ))
        .into_any_element()
}

fn body<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    visible: &[&Preset],
    cx: &mut Context<T>,
) -> AnyElement {
    if snapshot.presets.is_empty() {
        return empty_state(
            theme,
            Icon::Layers,
            "No presets",
            "A preset is a text file of package names, one per line. Drop one into assets/data/presets and it appears here.",
            None,
        );
    }
    if visible.is_empty() {
        return empty_state(
            theme,
            Icon::Search,
            "Nothing matches that filter",
            "Clear the search field to see every preset again.",
            None,
        );
    }

    let mut list = shared::scroll_body("presets-scroll");
    for preset in visible {
        let name = preset.name.clone();
        let actions = div()
            .flex()
            .flex_row()
            .items_center()
            .child(
                button(theme, format!("load-preset-{}", preset.name))
                    .label("Load into the queue")
                    .icon(Icon::ArrowRight)
                    .outline()
                    .disabled(snapshot.is_busy())
                    .build(cx.listener(move |this: &mut T, _event: &ClickEvent, _window, cx| {
                        this.load_preset(&name, cx)
                    })),
            )
            .into_any_element();
        list = list.child(rows::preset_row(theme, preset, actions));
    }

    panel_body().child(list).into_any_element()
}

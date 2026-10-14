//! The Scripts surface: the PowerShell library, grouped by intent.
//!
//! A script row carries prose, because the whole safety model depends on the user
//! having read what a script does before running it. So the row is two lines plus
//! the summary, and the summary is never truncated to one line.
//!
//! Grouping is by category rather than by risk: risk is a property *of* a row and
//! shows as a chip, while the category is where the file lives, and the shelves are
//! how someone remembers their own library.

use crate::components::button::{Variant, button};
use crate::components::panel::{panel_body, panel_header};
use crate::components::rows;
use crate::components::{Tone, chip, chip_with, empty_state, section_eyebrow, switch};
use crate::icons::Icon;
use crate::snapshot::{Screen, Snapshot};
use crate::theme::{Theme, metrics, space, veil};
use crate::views::{DebrisApp, shared};
use debris_core::{Category, ScriptMeta};
use gpui::{AnyElement, ClickEvent, Context, IntoElement, div, prelude::*, px};

pub fn screen<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    let visible: Vec<&ScriptMeta> = snapshot
        .scripts
        .iter()
        .filter(|script| matches(script, &snapshot.query))
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
            Screen::Scripts.title(),
            subtitle(snapshot, visible.len()),
            None,
        ))
        .child(toolbar::<T>(theme, snapshot, visible.len(), cx))
        .child(body::<T>(theme, snapshot, &visible, cx))
        .into_any_element()
}

fn matches(script: &ScriptMeta, query: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }
    script.id.to_ascii_lowercase().contains(&query)
        || script.title.to_ascii_lowercase().contains(&query)
        || script.summary_text().to_ascii_lowercase().contains(&query)
        || script.category.label().to_ascii_lowercase().contains(&query)
}

fn subtitle(snapshot: &Snapshot, shown: usize) -> String {
    let total = snapshot.scripts.len();
    // Say "of" only when the search field is actually narrowing something; otherwise
    // the two numbers are the same number said twice.
    let scope = if shown == total {
        shared::count(total, "script", "scripts")
    } else {
        format!("{shown} of {total} shown")
    };
    if snapshot.dry_run {
        format!("{scope} · previewing")
    } else {
        format!("{scope} · preview off")
    }
}

fn toolbar<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    shown: usize,
    cx: &mut Context<T>,
) -> AnyElement {
    let dry_run = snapshot.dry_run;

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::SM))
        .h(px(metrics::TOOLBAR))
        .w_full()
        .flex_shrink_0()
        .px(px(space::MD))
        .bg(theme.band(veil::BAND_QUIET))
        .border_b_1()
        .border_color(theme.border)
        .child(switch(
            theme,
            "preview-scripts",
            dry_run,
            "Preview with -DryRun",
            cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                let next = !this.snapshot().dry_run;
                this.set_dry_run(next, cx);
            }),
        ))
        // When preview is off this is the most important sentence on the screen, so
        // it stops being a muted note and becomes a warning chip.
        .when(!dry_run, |row| {
            row.child(chip_with(
                theme,
                Some(Icon::Alert),
                "Preview off: running a script will change this machine",
                Tone::Danger,
            ))
        })
        .child(div().flex_1())
        .child(chip(theme, format!("{shown} shown"), Tone::Quiet))
        .into_any_element()
}

fn body<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    visible: &[&ScriptMeta],
    cx: &mut Context<T>,
) -> AnyElement {
    if snapshot.scripts.is_empty() {
        return empty_state(
            theme,
            Icon::Terminal,
            "No scripts found",
            "Script library is empty.",
            None,
        );
    }
    if visible.is_empty() {
        return empty_state(
            theme,
            Icon::Search,
            "Nothing matches that filter",
            "Clear the search field to see the whole library again.",
            None,
        );
    }

    let mut list = shared::scroll_body("scripts-scroll");
    for category in Category::ALL {
        let group: Vec<&&ScriptMeta> = visible
            .iter()
            .filter(|script| script.category == category)
            .collect();
        if group.is_empty() {
            continue;
        }
        list = list.child(section_eyebrow(
            theme,
            category.label(),
            Some(group.len()),
        ));
        for script in group {
            let actions = row_actions::<T>(theme, script, snapshot, cx);
            list = list.child(rows::script_row(theme, script, actions));
        }
    }

    panel_body().child(list).into_any_element()
}

fn row_actions<T: DebrisApp>(
    theme: &Theme,
    script: &ScriptMeta,
    snapshot: &Snapshot,
    cx: &mut Context<T>,
) -> AnyElement {
    let id = script.id.clone();
    let dry_run = snapshot.dry_run;
    // A script the library refused is not offered, and says why in its own row.
    let enabled = script.is_runnable() && !snapshot.is_busy();

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::SM))
        .child(
            button(theme, format!("run-script-{}", script.id))
                .label(if dry_run { "Preview" } else { "Run" })
                .icon(if dry_run { Icon::Eye } else { Icon::Alert })
                .variant(if dry_run {
                    Variant::Outline
                } else {
                    Variant::Danger
                })
                .disabled(!enabled)
                .build(cx.listener(move |this: &mut T, _event: &ClickEvent, window, cx| {
                    this.request_script(&id, window, cx)
                })),
        )
        .into_any_element()
}

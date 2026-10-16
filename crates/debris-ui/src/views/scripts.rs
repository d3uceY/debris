//! The Scripts surface: the PowerShell library, grouped by intent.
//!
//! A script row carries prose, because the whole safety model depends on the user
//! having read what a script does before running it. So the row is two lines plus
//! the summary, and the summary is never truncated to one line.
//!
//! Grouping is by category rather than by risk: risk is a property *of* a row and
//! shows as a chip, while the category is where the file lives, and the shelves are
//! how someone remembers their own library.
//!
//! # Why a tick set rather than a button per row
//!
//! A run button repeated down a list makes each script look like a small, separate
//! act. It is not: the user arrives with a job in mind and picks the scripts that do
//! it. Ticking is that decision, and the footer states the size of it before anything
//! runs.

use crate::components::button::{Variant, button};
use crate::components::panel::{panel_body, panel_footer, panel_header};
use crate::components::rows;
use crate::components::{Tone, chip, empty_state, section_eyebrow};
use crate::icons::Icon;
use crate::snapshot::{Screen, Snapshot};
use crate::theme::{Theme, metrics, space, text, veil};
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
        .child(toolbar::<T>(theme, snapshot, cx))
        .child(body::<T>(theme, snapshot, &visible, cx))
        .child(footer::<T>(theme, snapshot, cx))
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
    let selected = snapshot.script_selection.len();
    let mut parts = vec![shared::count(total, "script", "scripts")];
    if selected > 0 {
        parts.push(format!("{selected} ticked"));
    }
    // Say "of" only when the search field is actually narrowing something; otherwise
    // the two numbers are the same number said twice.
    if shown != total {
        parts.push(format!("{shown} shown"));
    }
    parts.join(" · ")
}

fn toolbar<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    let busy = snapshot.is_busy();
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
        .child(
            button(theme, "scripts-tick-all")
                .label("Tick all")
                .ghost()
                .small()
                .disabled(busy || snapshot.scripts.is_empty())
                .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.tick_all_scripts(true, cx)
                })),
        )
        .child(
            button(theme, "scripts-tick-none")
                .label("Tick none")
                .ghost()
                .small()
                .disabled(busy || snapshot.script_selection.is_empty())
                .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.tick_all_scripts(false, cx)
                })),
        )
        .child(div().flex_1())
        .child(chip(
            theme,
            format!("{} ticked", snapshot.script_selection.len()),
            Tone::Quiet,
        ))
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
            "This build ships no script library.",
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
        list = list.child(section_eyebrow(theme, category.label(), Some(group.len())));
        for script in group {
            let ticked = snapshot.script_is_selected(&script.id);
            let id = script.id.clone();
            list = list.child(rows::script_row(
                theme,
                script,
                ticked,
                cx.listener(move |this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.toggle_script(&id, cx)
                }),
                trailing(theme, script),
            ));
        }
    }

    panel_body().child(list).into_any_element()
}

/// The trailing slot says why a script is not selectable, when it is not. A script
/// the library refused is still readable, just not runnable.
fn trailing(theme: &Theme, script: &ScriptMeta) -> AnyElement {
    if let Some(reason) = script.blocked_reason() {
        return chip(theme, reason, Tone::Danger).into_any_element();
    }
    div().into_any_element()
}

/// The ask: one line naming the size of the run, one button.
fn footer<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    let selected = snapshot.selected_scripts();
    let ticked = selected.len();
    let busy = snapshot.is_busy();
    let destructive = selected.iter().any(|script| script.risk.is_destructive());

    panel_footer(theme)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(space::SM))
                .flex_1()
                .min_w_0()
                .child(if ticked == 0 {
                    chip(theme, "nothing ticked", Tone::Quiet)
                } else {
                    chip(theme, format!("{ticked} ticked"), Tone::Neutral)
                })
                .when(destructive, |row| {
                    row.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(text::LABEL))
                            .font_family(theme.font_ui.clone())
                            .text_color(theme.danger)
                            .child("One of these declares destructive risk. Read it before running."),
                    )
                }),
        )
        .child(
            button(theme, "scripts-run")
                .label(if ticked == 0 {
                    "Run scripts".to_string()
                } else {
                    format!("Run {ticked} script(s)")
                })
                .variant(if destructive {
                    Variant::Danger
                } else {
                    Variant::Primary
                })
                .disabled(busy || ticked == 0)
                .build(cx.listener(|this: &mut T, _event: &ClickEvent, window, cx| {
                    this.request_scripts(window, cx)
                })),
        )
        .into_any_element()
}

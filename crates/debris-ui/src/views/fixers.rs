//! The Fixers surface: registry rules, ticked and run as a set.
//!
//! This is the surface the reference tool is built around, and the shape is the point:
//! a list you can read top to bottom, a box per rule, and one button that writes what
//! is ticked. Everything before that button is *information* — what the rule changes,
//! whether the machine already satisfies it, what it holds right now — because the
//! decision being made is "do I want this", and that needs the facts in front of it.
//!
//! # Why settings are checked, then run, and never applied on tick
//!
//! Ticking a box writes nothing. That separation is the whole safety model here: the
//! catalogue can be browsed, filtered and ticked freely, and the one irreversible act
//! is a button that also says how many rules it is about to write.

use crate::components::button::{Variant, button};
use crate::components::panel::{panel_body, panel_footer, panel_header};
use crate::components::rows;
use crate::components::{Tone, chip, empty_state, section_eyebrow, switch};
use crate::icons::Icon;
use crate::snapshot::{Screen, Snapshot};
use crate::theme::{Theme, metrics, space, text, veil};
use crate::views::{DebrisApp, shared};
use debris_core::{Tweak, TweakStatus};
use gpui::{AnyElement, ClickEvent, Context, IntoElement, div, prelude::*, px};

pub fn screen<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    let visible: Vec<&Tweak> = snapshot
        .tweaks
        .iter()
        .filter(|tweak| matches(tweak, &snapshot.query))
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
            Screen::Fixers.title(),
            subtitle(snapshot, visible.len()),
            None,
        ))
        .child(toolbar::<T>(theme, snapshot, cx))
        .child(body::<T>(theme, snapshot, &visible, cx))
        .child(footer::<T>(theme, snapshot, cx))
        .into_any_element()
}

fn matches(tweak: &Tweak, query: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }
    tweak.name.to_ascii_lowercase().contains(&query)
        || tweak.description.to_ascii_lowercase().contains(&query)
        || tweak.category_label().to_ascii_lowercase().contains(&query)
        || tweak
            .primary()
            .is_some_and(|entry| entry.value_name.to_ascii_lowercase().contains(&query))
}

fn subtitle(snapshot: &Snapshot, shown: usize) -> String {
    let total = snapshot.tweaks.len();
    let selected = snapshot.selected_tweaks().len();
    // The count of what is ticked comes before how much is on screen: it is the
    // number the Run button is about to act on.
    let mut parts = vec![shared::count(total, "fixer", "fixers")];
    if selected > 0 {
        parts.push(format!("{selected} ticked"));
    }
    if shown != total {
        parts.push(format!("{shown} shown"));
    }
    parts.join(" · ")
}

fn toolbar<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    let busy = snapshot.is_busy();
    let unfixed = snapshot
        .tweaks
        .iter()
        .filter(|tweak| snapshot.tweak_scan(&tweak.name).status == TweakStatus::NotApplied)
        .count();
    // A non-empty map is the same statement as "something has been checked", and it
    // costs nothing to ask.
    let scanned = !snapshot.tweak_scans.is_empty();

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
            button(theme, "fixers-tick-all")
                .label("Tick all")
                .ghost()
                .small()
                .disabled(busy || snapshot.tweaks.is_empty())
                .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.tick_all_fixers(true, cx)
                })),
        )
        .child(
            button(theme, "fixers-tick-none")
                .label("Tick none")
                .ghost()
                .small()
                .disabled(busy || snapshot.tweak_selection.is_empty())
                .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.tick_all_fixers(false, cx)
                })),
        )
        .child(
            button(theme, "fixers-analyze")
                .label("Check")
                .icon(Icon::Search)
                .outline()
                .small()
                .disabled(busy || snapshot.tweaks.is_empty())
                .tooltip("Read what the registry holds for every fixer")
                .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.analyze_fixers(cx)
                })),
        )
        .child(div().flex_1())
        .when(scanned, |bar| {
            bar.child(chip(
                theme,
                format!("{unfixed} not set"),
                if unfixed == 0 { Tone::Success } else { Tone::Danger },
            ))
        })
        .when(!scanned, |bar| bar.child(chip(theme, "not checked", Tone::Neutral)))
        // The backup is a real decision with a real cost, so it is a control rather
        // than a line in a settings file nobody opens.
        .child(switch(
            theme,
            "fixers-backup",
            snapshot.backup_before_apply,
            "Write a .reg backup",
            cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                this.toggle_backup(cx)
            }),
        ))
        .into_any_element()
}

fn body<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    visible: &[&Tweak],
    cx: &mut Context<T>,
) -> AnyElement {
    if snapshot.tweaks.is_empty() {
        return empty_state(
            theme,
            Icon::Wrench,
            "No fixers found",
            "This build ships no fixer catalogue.",
            None,
        );
    }
    if visible.is_empty() {
        return empty_state(
            theme,
            Icon::Search,
            "Nothing matches that filter",
            "Clear the search field to see the whole catalogue.",
            None,
        );
    }

    let mut list = shared::scroll_body("fixers-scroll");
    // Catalogue order, so the groups read in the order they were written rather than
    // in whatever order a hash happened to produce.
    let mut categories: Vec<&str> = Vec::new();
    for tweak in visible {
        let category = tweak.category_label();
        if !categories.contains(&category) {
            categories.push(category);
        }
    }

    for category in categories {
        let group: Vec<&&Tweak> = visible
            .iter()
            .filter(|tweak| tweak.category_label() == category)
            .collect();
        list = list.child(section_eyebrow(theme, category, Some(group.len())));
        for tweak in group {
            let ticked = snapshot.tweak_selection.contains(&tweak.name);
            let scan = snapshot.tweak_scan(&tweak.name);
            // Cloned because the handler has to outlive this frame.
            let name = tweak.name.clone();
            list = list.child(rows::tweak_row(
                theme,
                tweak,
                ticked,
                scan.status,
                scan.current.as_deref(),
                cx.listener(move |this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.toggle_fixer(&name, cx)
                }),
            ));
        }
    }

    panel_body().child(list).into_any_element()
}

/// The ask. Both controls here write to the registry, so both are named for what
/// they do and neither is a ghost.
fn footer<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    let selected = snapshot.selected_tweaks();
    let ticked = selected.len();
    let busy = snapshot.is_busy();
    let restorable = selected.iter().filter(|tweak| tweak.is_restorable()).count();
    let needs_admin = selected.iter().any(|tweak| tweak.requires_admin);
    // Only warn when it would actually stop the run. An administrator sees nothing.
    let elevation_note = (needs_admin && snapshot.elevated == Some(false)).then_some(
        "Some of these write outside your user account. Run debris as administrator, or they will fail.",
    );

    let summary = if ticked == 0 {
        chip(theme, "nothing ticked", Tone::Quiet)
    } else {
        chip(
            theme,
            format!("{ticked} of {} ticked", snapshot.tweaks.len()),
            Tone::Neutral,
        )
    };

    panel_footer(theme)
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(space::SM))
                .flex_1()
                .min_w_0()
                .child(summary)
                .when_some(elevation_note, |row, note| {
                    row.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(px(text::LABEL))
                            .font_family(theme.font_ui.clone())
                            .text_color(theme.warning)
                            .child(note),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(space::SM))
                .child(
                    button(theme, "fixers-restore")
                        .label("Restore defaults")
                        .outline()
                        .disabled(busy || restorable == 0)
                        .tooltip("Put the ticked fixers back to the value Windows ships with")
                        .build(cx.listener(|this: &mut T, _event: &ClickEvent, window, cx| {
                            this.request_restore(window, cx)
                        })),
                )
                .child(
                    button(theme, "fixers-run")
                        .label(if ticked == 0 {
                            "Run fixers".to_string()
                        } else {
                            format!("Run {ticked} fixer(s)")
                        })
                        .variant(if ticked == 0 {
                            Variant::Outline
                        } else {
                            Variant::Primary
                        })
                        .disabled(busy || ticked == 0)
                        .build(cx.listener(|this: &mut T, _event: &ClickEvent, window, cx| {
                            this.request_fixers(window, cx)
                        })),
                ),
        )
        .into_any_element()
}

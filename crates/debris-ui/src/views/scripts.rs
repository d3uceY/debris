//! The Scripts surface: the PowerShell library, grouped by intent.

use crate::components::button::{ButtonVariant, button};
use crate::components::rows;
use crate::components::{empty_state, label, rule};
use crate::icons::{Icon, icon};
use crate::snapshot::{Screen, Snapshot};
use crate::theme::{Theme, text};
use crate::views::{DebrisApp, section_title};
use debris_core::{Category, ScriptMeta};
use gpui::{AnyElement, ClickEvent, Context, IntoElement, div, prelude::*, px};

pub fn screen<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    // `overflow_scroll` lives on `StatefulInteractiveElement`, so the container needs
    // an element id before it can scroll.
    let mut list = div()
        .id("scripts-scroll")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .overflow_scroll();

    if snapshot.scripts.is_empty() {
        list = list.child(empty_state(
            theme,
            "No scripts found",
            "The library lives in scripts/ next to the project. Add a *.ps1 file with a ### header and it appears here.",
        ));
    } else {
        for category in Category::ALL {
            let group: Vec<&ScriptMeta> = snapshot
                .scripts
                .iter()
                .filter(|script| script.category == category)
                .collect();
            if group.is_empty() {
                continue;
            }
            list = list.child(category_header(theme, category, group.len()));
            for script in group {
                let actions = row_actions::<T>(theme, script, snapshot, cx);
                list = list.child(rows::script_row(theme, script, actions));
            }
        }
    }

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .child(section_title(theme, Screen::Scripts))
        .child(rule(theme))
        .child(preview_notice(theme, snapshot))
        .child(rule(theme))
        .child(list)
        .into_any_element()
}

/// One line, stating the mode the whole surface is in. When preview is off this is
/// the most important sentence on the screen, so it turns the accent colour.
fn preview_notice(theme: &Theme, snapshot: &Snapshot) -> AnyElement {
    let dry_run = snapshot.dry_run;
    div()
        .flex()
        .items_center()
        .gap_3()
        .px_6()
        .py_3()
        .flex_shrink_0()
        .text_color(if dry_run { theme.ink_muted } else { theme.accent })
        .child(icon(
            if dry_run { Icon::Shield } else { Icon::Warning },
            13.0,
        ))
        .child(
            div()
                .font_family(theme.font_ui.clone())
                .text_size(px(text::SMALL))
                .child(if dry_run {
                    "Scripts run with -DryRun: they list what they would do and change nothing."
                } else {
                    "Preview is off. Running a script will change this machine."
                }),
        )
        .into_any_element()
}

fn category_header(theme: &Theme, category: Category, count: usize) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_3()
        .px_4()
        .py_3()
        .bg(theme.surface)
        .border_b_1()
        .border_color(theme.rule_strong)
        .child(label(theme, category.label()))
        .child(
            div()
                .font_family(theme.font_mono.clone())
                .text_size(px(text::LABEL))
                .text_color(theme.ink_faint)
                .child(count.to_string()),
        )
        .into_any_element()
}

fn row_actions<T: DebrisApp>(
    theme: &Theme,
    script: &ScriptMeta,
    snapshot: &Snapshot,
    cx: &mut Context<T>,
) -> AnyElement {
    let id = script.id.clone();
    let dry_run = snapshot.dry_run;
    // A script the library refused is not offered, and says why in its row.
    let enabled = script.is_runnable() && snapshot.loading.is_none();

    div()
        .flex()
        .items_center()
        .gap_3()
        .child(button(
            theme,
            format!("run-script-{}", script.id),
            if dry_run { "Preview" } else { "Run" },
            if dry_run {
                ButtonVariant::Default
            } else {
                ButtonVariant::Danger
            },
            enabled,
            cx.listener(move |this: &mut T, _event: &ClickEvent, _window, cx| {
                this.run_script(&id, cx)
            }),
        ))
        .child(
            div()
                .font_family(theme.font_mono.clone())
                .text_size(px(text::LABEL))
                .text_color(theme.ink_faint)
                .child(if dry_run { "-DryRun" } else { "live" }),
        )
        .into_any_element()
}

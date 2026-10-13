//! The removal queue — the left column.
//!
//! This is the signature region of the app, so it is on screen on every surface and
//! never moves. A package crosses from the center table into this column, and the
//! count in its header ticks over. Nothing about the crossing changes the row's
//! identity except its ink: the mark becomes a check, the wash becomes the removal
//! colour, and the header count goes up by one.
//!
//! The footer is the confirmation. It states the plan in one line, names the caveat
//! when there is one, and holds the only button in the app that can destroy
//! anything.

use crate::components::button::{Variant, button};
use crate::components::panel::panel_footer;
use crate::components::rows::queue_row;
use crate::components::sidebar::{column_body, column_header, side_column};
use crate::components::pane_hint;
use crate::icons::Icon;
use crate::snapshot::Snapshot;
use crate::theme::{Theme, space, text, veil};
use crate::views::DebrisApp;
use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, IntoElement, WeakEntity, Window, div, prelude::*,
    px,
};

pub fn ledger<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    root: &WeakEntity<T>,
    cx: &mut Context<T>,
) -> AnyElement {
    let busy = snapshot.is_busy();
    let count = snapshot.staged.len();

    let header = column_header(
        theme,
        "TO REMOVE",
        Some(count),
        Some(
            button(theme, "hide-queue")
                .ghost()
                .small()
                .icon(Icon::PanelLeft)
                .icon_only()
                .tooltip("Hide the removal queue")
                .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.toggle_sidebar(cx)
                })),
        ),
    );

    let body: AnyElement = if snapshot.staged.is_empty() {
        pane_hint(
            theme,
            "Nothing staged. Click a package to move it into this column.",
        )
    } else {
        let mut list = column_body().id("queue-scroll").overflow_scroll();
        for (index, package) in snapshot.staged.iter().enumerate() {
            let name = package.name.clone();
            let entity = root.clone();
            list = list.child(queue_row(
                theme,
                ("queue", index),
                package,
                move |_event: &ClickEvent, _window: &mut Window, cx: &mut gpui::App| {
                    let name = name.clone();
                    entity.update(cx, |this, cx| this.unstage(&name, cx)).ok();
                },
            ));
        }
        list.into_any_element()
    };

    let plan = &snapshot.plan;

    // Two different sentences with two different jobs: the line above the buttons is
    // the *state* (prose, past tense of what has been gathered), and the button is
    // the *action*. Repeating the button's label as the summary above it — which is
    // what `plan.headline()` alone would do — says the same thing twice and makes the
    // empty case read like a stutter.
    let summary = match plan.len() {
        0 => "Nothing staged.".to_string(),
        1 => "1 package queued.".to_string(),
        count => format!("{count} packages queued."),
    };
    let action = if plan.is_empty() {
        "Remove packages".to_string()
    } else {
        plan.headline()
    };

    let mut footer = panel_footer(theme).child(
        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .font_family(theme.font_ui.clone())
                    .text_size(px(text::SUBTITLE))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if plan.is_empty() {
                        theme.muted_foreground
                    } else {
                        theme.foreground
                    })
                    .child(summary),
            ),
    );

    if let Some(note) = plan.caveat_note() {
        footer = footer.child(
            div()
                .flex()
                .flex_row()
                .items_start()
                .gap(px(space::XS))
                .child(crate::icons::icon(Icon::Alert, 13.0, theme.warning))
                .child(
                    div()
                        .font_family(theme.font_ui.clone())
                        .text_size(px(text::LABEL))
                        .child(note),
                ),
        );
    }

    let run_variant = if plan.requires_acknowledgement() {
        Variant::Danger
    } else {
        Variant::Primary
    };

    let footer = footer
        .child(
            div()
                .font_family(theme.font_ui.clone())
                .text_size(px(text::LABEL))
                .text_color(theme.muted_foreground)
                .child("Nothing runs until you press the button."),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(space::SM))
                .child(
                    div().flex_1().child(
                        button(theme, "run-removal")
                            .label(action)
                            .variant(run_variant)
                            .disabled(plan.is_empty() || busy)
                            .build(cx.listener(|this: &mut T, _event: &ClickEvent, window, cx| {
                                this.request_removal(window, cx)
                            })),
                    ),
                )
                .child(
                    button(theme, "clear-queue")
                        .label("Clear")
                        .danger_ghost()
                        .disabled(plan.is_empty() || busy)
                        .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                            this.unstage_all(cx)
                        })),
                ),
        );

    side_column(theme)
        .child(header)
        .child(body)
        .child(footer)
        .child(
            // A hairline of the removal colour at the very foot of the column. It is
            // the one place the accent is decoration, and it exists so the column is
            // never mistaken for an ordinary sidebar.
            div()
                .w_full()
                .h(px(2.0))
                .flex_shrink_0()
                .bg(if plan.is_empty() {
                    theme.border.opacity(veil::HAIRLINE)
                } else {
                    theme.danger
                }),
        )
        .into_any_element()
}

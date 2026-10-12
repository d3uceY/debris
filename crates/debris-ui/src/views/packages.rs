//! The Packages surface — the primary one.
//!
//! Two columns over one hairline: the inventory on the left, the ledger of what will
//! go on the right. The same row component is used in both, so a package that is
//! staged is visibly the *same row* in a different place rather than a different
//! looking thing.

use crate::components::button::{ButtonVariant, button};
use crate::components::text_field::text_field;
use crate::components::{checkbox, empty_state, error_state, label, loading_bar, rule};
use crate::icons::Icon;
use crate::snapshot::{Screen, Snapshot};
use crate::theme::{Theme, layout, text};
use crate::views::{DebrisApp, list_key_handler, package_list, section_title};
use gpui::{
    AnyElement, ClickEvent, Context, FocusHandle, FontWeight, IntoElement, KeyDownEvent,
    MouseButton, MouseDownEvent, WeakEntity, div, prelude::*, px,
};

pub fn screen<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    root: &WeakEntity<T>,
    search_focus: &FocusHandle,
    list_focus: &FocusHandle,
    cx: &mut Context<T>,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .child(section_title(theme, Screen::Packages))
        .child(rule(theme))
        .child(toolbar::<T>(theme, snapshot, search_focus, cx))
        .child(rule(theme))
        .child(
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .child(inventory::<T>(theme, snapshot, root, list_focus, cx))
                .child(removal::<T>(theme, snapshot, root, cx)),
        )
        .into_any_element()
}

fn toolbar<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    search_focus: &FocusHandle,
    cx: &mut Context<T>,
) -> AnyElement {
    let shown = snapshot.inventory_len();

    div()
        .flex()
        .items_center()
        .gap_4()
        .px_6()
        .py_3()
        .flex_shrink_0()
        .child(
            div().w(px(340.0)).flex_shrink_0().child(text_field(
                theme,
                "package-filter",
                &snapshot.query,
                "Filter by name or publisher",
                search_focus,
                cx.listener(|this: &mut T, event: &KeyDownEvent, _window, cx| {
                    let current = this.snapshot().query.clone();
                    match event.keystroke.key.as_str() {
                        "backspace" => {
                            let mut next = current;
                            next.pop();
                            this.set_query(next, cx);
                        }
                        "escape" | "delete" => this.set_query(String::new(), cx),
                        _ => {
                            if let Some(typed) = event.keystroke.key_char.as_deref() {
                                if !typed.is_empty() && !typed.chars().any(char::is_control) {
                                    let mut next = current;
                                    next.push_str(typed);
                                    this.set_query(next, cx);
                                }
                            }
                        }
                    }
                }),
            )),
        )
        .child(checkbox(
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
        .child(label(theme, format!("{shown} shown")))
        .child(button(
            theme,
            "stage-all-visible",
            "Stage all shown",
            ButtonVariant::Default,
            shown > 0,
            cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| this.stage_all(cx)),
        ))
        .into_any_element()
}

/// The inventory column. Its own focus handle, so arrow keys move a cursor through
/// the list without stealing focus from the search field.
fn inventory<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    root: &WeakEntity<T>,
    list_focus: &FocusHandle,
    cx: &mut Context<T>,
) -> AnyElement {
    let mut column = div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .track_focus(list_focus)
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this: &mut T, _event: &MouseDownEvent, window, cx| {
                window.focus(this.list_focus(), cx);
            }),
        )
        .on_key_down(list_key_handler(cx));

    if snapshot.loading.is_some() {
        column = column.child(loading_bar(theme, "inventory-loading"));
    }

    // Four different nothings, four different sentences. "Empty" is only truthful
    // once we know *why* it is empty.
    let body: AnyElement = if let Some(error) = &snapshot.error {
        error_state(
            theme,
            "The inventory could not be read",
            &format!("{error}\nIt usually means PowerShell could not reach the Appx store. Run Rescan to try again."),
        )
        .into_any_element()
    } else if snapshot.inventory_is_empty() && !snapshot.scanned {
        empty_state(
            theme,
            "No inventory yet",
            "Press Rescan to ask Windows what is installed for this user.",
        )
        .into_any_element()
    } else if snapshot.inventory_is_empty() && snapshot.filter_is_active() {
        empty_state(
            theme,
            "Nothing matches that filter",
            "Clear the filter to see the whole inventory again.",
        )
        .into_any_element()
    } else if snapshot.inventory_is_empty() {
        empty_state(
            theme,
            "Every package is staged",
            "Everything this user can uninstall is in the column on the right. Nothing has been removed yet.",
        )
        .into_any_element()
    } else {
        package_list::<T>(
            theme,
            "inventory",
            snapshot.inventory.clone(),
            snapshot.staged_names.clone(),
            false,
            Some(snapshot.cursor),
            root,
        )
    };

    column.child(body).into_any_element()
}

/// The ledger, and the confirm step. The confirm is part of the column rather than a
/// dialog: removal is a decision worth keeping on screen, not worth interrupting for.
fn removal<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    root: &WeakEntity<T>,
    cx: &mut Context<T>,
) -> AnyElement {
    let plan = &snapshot.plan;
    let busy = snapshot.loading.is_some();

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .h(px(layout::ROW))
        .px_4()
        .flex_shrink_0()
        .border_b_1()
        .border_color(theme.rule)
        // The arrow is the hint that a click returns the package: it is the same
        // gesture in reverse, so it gets the same mark.
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .text_color(theme.ink_faint)
                .child(crate::icons::icon(Icon::ArrowLeft, 11.0))
                .child(label(theme, "To remove")),
        )
        .child(
            div()
                .font_family(theme.font_mono.clone())
                .text_size(px(text::SMALL))
                .text_color(theme.ink)
                .child(snapshot.counts.staged.to_string()),
        );

    let list: AnyElement = if snapshot.staged.is_empty() {
        div().p_4().child(
            div()
                .font_family(theme.font_ui.clone())
                .text_size(px(text::SMALL))
                .text_color(theme.ink_muted)
                .child("Nothing staged. Click a package to move it here."),
        )
        .into_any_element()
    } else {
        package_list::<T>(
            theme,
            "staged",
            snapshot.staged.clone(),
            snapshot.staged_names.clone(),
            true,
            None,
            root,
        )
    };

    let mut footer = div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .flex_shrink_0()
        .bg(theme.surface_sunken)
        .border_t_1()
        .border_color(theme.rule_strong)
        .child(
            div()
                .font_family(theme.font_ui.clone())
                .text_size(px(text::SUBTITLE))
                .font_weight(FontWeight::MEDIUM)
                .text_color(if plan.is_empty() { theme.ink_muted } else { theme.ink })
                .child(plan.headline()),
        );

    if let Some(note) = plan.caveat_note() {
        footer = footer.child(
            div()
                .flex()
                .items_start()
                .gap_2()
                .text_color(theme.warning)
                .child(crate::icons::icon(Icon::Warning, 13.0))
                .child(
                    div()
                        .font_family(theme.font_ui.clone())
                        .text_size(px(text::SMALL))
                        .child(note),
                ),
        );
    }

    footer = footer
        .child(
            div()
                .font_family(theme.font_ui.clone())
                .text_size(px(text::LABEL))
                .text_color(theme.ink_muted)
                .child("Nothing runs until you press the button."),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(button(
                    theme,
                    "run-removal",
                    if plan.is_empty() {
                        "Nothing staged".to_string()
                    } else {
                        plan.headline()
                    },
                    if plan.requires_acknowledgement() {
                        ButtonVariant::Danger
                    } else {
                        ButtonVariant::Primary
                    },
                    !plan.is_empty() && !busy,
                    cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                        this.run_removal(cx)
                    }),
                ))
                .child(button(
                    theme,
                    "clear-staged",
                    "Clear",
                    ButtonVariant::Quiet,
                    !plan.is_empty(),
                    cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                        this.unstage_all(cx)
                    }),
                )),
        );

    div()
        .flex()
        .flex_col()
        .w(px(layout::REMOVAL_COLUMN))
        .flex_shrink_0()
        .min_h_0()
        .bg(theme.surface_sunken)
        .border_l_1()
        .border_color(theme.rule)
        .child(header)
        .child(list)
        .child(footer)
        .into_any_element()
}

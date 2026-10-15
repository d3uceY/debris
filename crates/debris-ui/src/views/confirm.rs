//! The second confirmation.
//!
//! One destructive action, asked about twice. The control that starts it lives where
//! the decision is made — the queue's footer, or a script's own row — and this dialog
//! is the step between that click and the change. It states exactly what will happen,
//! names the caveat when there is one, and holds the only control that commits.
//!
//! It floats rather than sitting inline because the two actions it guards live in two
//! different regions of the shell; an inline confirm could not be in both places
//! without becoming two different confirmations. It keeps the app's language: no
//! radius on the container, the same bands, the same two text colours. The one thing
//! it adds is a scrim — the depth system doing the job it exists for.

use crate::components::button::{Variant, button};
use crate::components::panel::{panel_footer, panel_header};
use crate::icons::{Icon, icon};
use crate::snapshot::{Pending, Snapshot};
use crate::theme::{Theme, space, text};
use crate::views::DebrisApp;
use debris_core::{ScriptMeta, Tweak};
use gpui::{
    AnyElement, ClickEvent, Context, FocusHandle, IntoElement, KeyDownEvent, div, prelude::*, px,
};

/// The dialog, or nothing if no action is waiting.
pub fn confirm_overlay<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    pending: &Pending,
    focus: &FocusHandle,
    cx: &mut Context<T>,
) -> AnyElement {
    let Confirm { title, subtitle, variant, confirm_label, body } =
        copy(theme, snapshot, pending);

    let panel = div()
        .id("confirm-panel")
        .flex()
        .flex_col()
        // Full width up to the dialog's own measure, so a window narrower than 440
        // squeezes the panel instead of clipping it against both edges.
        .w_full()
        .max_w(px(440.0))
        .bg(theme.background)
        .border_1()
        .border_color(theme.border)
        .shadow(theme.overlay_shadow())
        .track_focus(focus)
        // Enter commits and Escape backs out, so the keyboard can answer the question
        // the dialog is asking. Both go through the same app events as the buttons.
        .on_key_down(cx.listener(|this: &mut T, event: &KeyDownEvent, _window, cx| {
            match event.keystroke.key.as_str() {
                "escape" => this.cancel_pending(cx),
                "enter" => this.confirm_pending(cx),
                _ => {}
            }
        }))
        .child(panel_header(theme, title, subtitle, None))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(space::SM))
                .p(px(space::MD))
                .children(body),
        )
        .child(
            panel_footer(theme).child(
                div()
                    .flex()
                    .flex_row()
                    .justify_end()
                    .gap(px(space::SM))
                    .child(
                        button(theme, "confirm-cancel")
                            .label("Cancel")
                            .outline()
                            .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                                this.cancel_pending(cx)
                            })),
                    )
                    .child(
                        button(theme, "confirm-go")
                            .label(confirm_label)
                            .variant(variant)
                            .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                                this.confirm_pending(cx)
                            })),
                    ),
            ),
        );

    // The scrim paints the ink and swallows the pointer, so nothing behind the dialog
    // can be clicked while the question is open. It does not dismiss on click: a
    // destructive confirm should require a decision, not a stray click.
    div()
        .id("confirm-scrim")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        // The gutter is what the panel's `max_w` measures against, so the dialog can
        // never touch the window edge.
        .p(px(space::LG))
        .bg(theme.scrim())
        .occlude()
        .child(panel)
        .into_any_element()
}

/// Everything the dialog says and does, resolved from the pending action.
struct Confirm {
    title: String,
    subtitle: String,
    variant: Variant,
    confirm_label: String,
    body: Vec<AnyElement>,
}

/// The copy is composed from what the app already holds — the plan, the ticked
/// scripts, the ticked fixers — so the sentence and the button can never disagree
/// with what will actually run.
fn copy(theme: &Theme, snapshot: &Snapshot, pending: &Pending) -> Confirm {
    match pending {
        Pending::Removal => {
            let plan = &snapshot.plan;
            let mut body = vec![paragraph(
                theme,
                "They are uninstalled for the account you are signed in as. Nothing else on this machine changes.",
            )];
            if let Some(note) = plan.caveat_note() {
                body.push(caveat_row(theme, &note));
            }
            Confirm {
                title: format!("{}?", plan.headline()),
                subtitle: "Nothing has been removed yet.".to_string(),
                variant: Variant::Danger,
                confirm_label: plan.headline(),
                body,
            }
        }
        Pending::Scripts => {
            let selected = snapshot.selected_scripts();
            let count = selected.len();
            let destructive = selected.iter().filter(|s| s.risk.is_destructive()).count();
            let needs_admin = selected.iter().filter(|s| s.requires_admin).count();

            let mut body = Vec::new();
            for script in selected.iter().take(8) {
                body.push(script_line(theme, script));
            }
            if count > 8 {
                body.push(paragraph(theme, &format!("and {} more.", count - 8)));
            }
            if destructive > 0 {
                body.push(caveat_row(
                    theme,
                    &format!(
                        "{destructive} of these declare destructive risk. Anything they remove may not come back."
                    ),
                ));
            }
            if needs_admin > 0 && snapshot.elevated == Some(false) {
                body.push(caveat_row(
                    theme,
                    "Some need administrator rights, and debris is not elevated. Those will fail.",
                ));
            }

            Confirm {
                title: match count {
                    0 => "Nothing ticked?".to_string(),
                    1 => format!("Run {}?", selected[0].title),
                    _ => format!("Run {count} scripts?"),
                },
                subtitle: "Nothing has run yet.".to_string(),
                variant: if destructive > 0 {
                    Variant::Danger
                } else {
                    Variant::Primary
                },
                confirm_label: match count {
                    0 => "Run".to_string(),
                    1 => "Run script".to_string(),
                    _ => format!("Run {count}"),
                },
                body,
            }
        }
        Pending::Fixers => {
            let selected = snapshot.selected_tweaks();
            let count = selected.len();
            let needs_admin = selected.iter().filter(|t| t.requires_admin).count();
            let restart: Vec<&str> = selected
                .iter()
                .map(|tweak| tweak.restart.label())
                .filter(|label| *label != "no restart")
                .collect();

            let mut body = vec![paragraph(
                theme,
                "Each rule is written to the registry value it names. Nothing else is touched.",
            )];
            for tweak in selected.iter().take(6) {
                body.push(tweak_line(theme, tweak));
            }
            if count > 6 {
                body.push(paragraph(theme, &format!("and {} more.", count - 6)));
            }
            if snapshot.backup_before_apply {
                body.push(paragraph(
                    theme,
                    "A .reg backup of the current values is written first.",
                ));
            } else {
                body.push(caveat_row(theme, "No backup will be written. Restoring uses the Windows default instead."));
            }
            if needs_admin > 0 && snapshot.elevated == Some(false) {
                body.push(caveat_row(
                    theme,
                    &format!(
                        "{needs_admin} of these write outside your user account, and debris is not elevated. Those will fail."
                    ),
                ));
            }
            if !restart.is_empty() {
                body.push(paragraph(
                    theme,
                    "Some of these need a restart before they take effect.",
                ));
            }

            Confirm {
                title: match count {
                    1 => format!("Apply {}?", selected[0].name),
                    _ => format!("Apply {count} fixers?"),
                },
                subtitle: "Nothing has been written yet.".to_string(),
                variant: Variant::Primary,
                confirm_label: match count {
                    1 => "Apply fixer".to_string(),
                    _ => format!("Apply {count}"),
                },
                body,
            }
        }
        Pending::Restore => {
            let selected: Vec<_> = snapshot
                .selected_tweaks()
                .into_iter()
                .filter(|tweak| tweak.is_restorable())
                .collect();
            let count = selected.len();
            let mut body = vec![paragraph(
                theme,
                "Each ticked fixer goes back to the value Windows ships with, or is removed where Windows ships nothing.",
            )];
            for tweak in selected.iter().take(6) {
                body.push(tweak_line(theme, tweak));
            }
            if count > 6 {
                body.push(paragraph(theme, &format!("and {} more.", count - 6)));
            }

            Confirm {
                title: format!("Restore {count} fixers?"),
                subtitle: "Nothing has been written yet.".to_string(),
                variant: Variant::Outline,
                confirm_label: "Restore".to_string(),
                body,
            }
        }
        Pending::Extension(url) => {
            let entry = snapshot.extensions.iter().find(|entry| entry.url() == url);
            let mut body = vec![paragraph(
                theme,
                "This script was written by whoever published the catalogue. It has not been reviewed here.",
            )];

            // One pass over the Option: absent means the catalogue has changed under
            // the dialog, and there is nothing else to say about it.
            let (title, needs_admin) = match entry {
                Some(entry) => {
                    body.push(code_line(theme, entry.extension.name.clone()));
                    if !entry.source.is_empty() {
                        body.push(code_line(theme, entry.source.clone()));
                    }
                    if !entry.extension.description.is_empty() {
                        body.push(paragraph(theme, &entry.extension.description));
                    }
                    if entry.extension.sha256.is_none() {
                        body.push(caveat_row(
                            theme,
                            "The catalogue publishes no checksum, so this copy cannot be verified.",
                        ));
                    }
                    (
                        format!("Run {}?", entry.extension.name),
                        entry.extension.requires_admin,
                    )
                }
                None => {
                    body.push(paragraph(theme, "This entry is no longer in any catalogue."));
                    ("Run this extension?".to_string(), false)
                }
            };

            if needs_admin && snapshot.elevated == Some(false) {
                body.push(caveat_row(
                    theme,
                    "It needs administrator rights, and debris is not elevated. It may fail.",
                ));
            }

            Confirm {
                title,
                subtitle: "Nothing has run yet.".to_string(),
                variant: Variant::Danger,
                confirm_label: "Run extension".to_string(),
                body,
            }
        }
    }
}

/// One script, as a dialog line: the title, then the risk it declares.
fn script_line(theme: &Theme, script: &ScriptMeta) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::SM))
        .child(crate::icons::icon(
            Icon::Terminal,
            crate::icons::INLINE,
            theme.muted_foreground,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_family(theme.font_ui.clone())
                .text_size(px(text::SMALL))
                .text_color(theme.foreground)
                .child(script.title.clone()),
        )
        .child(crate::components::rows::risk_chip(theme, script.risk))
        .into_any_element()
}

/// One fixer, as a dialog line: the name and the value it will take.
fn tweak_line(theme: &Theme, tweak: &Tweak) -> AnyElement {
    let target = tweak
        .primary()
        .map(|entry| format!("{} = {}", entry.display_name(), entry.recommended))
        .unwrap_or_default();
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::SM))
        .child(crate::icons::icon(
            Icon::Wrench,
            crate::icons::INLINE,
            theme.muted_foreground,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_family(theme.font_ui.clone())
                .text_size(px(text::SMALL))
                .text_color(theme.foreground)
                .child(tweak.name.clone()),
        )
        .child(
            div()
                .flex_shrink_0()
                .font_family(theme.font_mono.clone())
                .text_size(px(text::LABEL))
                .text_color(theme.muted_foreground)
                .child(target),
        )
        .into_any_element()
}

fn paragraph(theme: &Theme, content: &str) -> AnyElement {
    div()
        .font_family(theme.font_ui.clone())
        .text_size(px(text::SMALL))
        .text_color(theme.foreground)
        .child(content.to_string())
        .into_any_element()
}

fn code_line(theme: &Theme, value: String) -> AnyElement {
    div()
        .font_family(theme.font_mono.clone())
        .text_size(px(text::LABEL))
        .text_color(theme.muted_foreground)
        .child(value)
        .into_any_element()
}

fn caveat_row(theme: &Theme, content: &str) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(space::XS))
        .child(icon(Icon::Alert, 13.0, theme.warning))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .font_family(theme.font_ui.clone())
                .text_size(px(text::LABEL))
                .text_color(theme.muted_foreground)
                .child(content.to_string()),
        )
        .into_any_element()
}

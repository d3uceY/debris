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

/// The copy is composed from what the app already holds — the plan and the script —
/// so the sentence and the button can never disagree with what will actually run.
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
        Pending::Script(id) => {
            let script = snapshot.scripts.iter().find(|script| &script.id == id);
            let mut body = Vec::new();
            if let Some(script) = script {
                body.push(paragraph(theme, script.risk.detail()));
                body.push(code_line(theme, script.id.clone()));
                if script.requires_admin {
                    body.push(caveat_row(
                        theme,
                        "This script needs administrator rights. Windows will ask before it runs.",
                    ));
                }
            } else {
                body.push(paragraph(theme, "This script is no longer in the library."));
            }
            Confirm {
                title: match script {
                    Some(script) => format!("Run {}?", script.title),
                    None => "Run this script?".to_string(),
                },
                subtitle: "Nothing has run yet.".to_string(),
                variant: match script {
                    Some(script) if script.risk.is_destructive() => Variant::Danger,
                    _ => Variant::Primary,
                },
                confirm_label: "Run script".to_string(),
                body,
            }
        }
    }
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

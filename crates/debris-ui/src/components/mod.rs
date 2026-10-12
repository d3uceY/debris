//! Reusable pieces of the interface.
//!
//! Every component takes `&Theme` rather than reaching for a global, so a component
//! can be rendered against either appearance in a test or a preview. Nothing here
//! holds state; behaviour arrives as a closure.

pub mod button;
pub mod rows;
pub mod status_bar;
pub mod text_field;

use crate::icons::{Icon, icon};
use crate::theme::{Theme, radius, text};
use gpui::{
    Animation, AnimationExt, AnyElement, App, ClickEvent, FontWeight, IntoElement, SharedString,
    Window, div, prelude::*, px, rgba,
};
use std::time::Duration;

/// The primary structural device in this interface. One pixel, no shadow. If a
/// region needs separating, this is how — not a card.
pub fn rule(theme: &Theme) -> impl IntoElement {
    div().w_full().h(px(1.0)).flex_shrink_0().bg(theme.rule)
}

/// Uppercase and small. It labels a region or a value; it is never stacked above a
/// heading as an eyebrow.
pub fn label(theme: &Theme, content: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_size(px(text::LABEL))
        .font_family(theme.font_ui.clone())
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.ink_faint)
        .child(content.into())
}

/// A count over a caption. The only numbers that get size.
pub fn stat(theme: &Theme, value: impl Into<SharedString>, caption: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .font_family(theme.font_mono.clone())
                .text_size(px(theme::STAT_SIZE))
                .text_color(theme.ink)
                .child(value.into()),
        )
        .child(label(theme, caption.to_string()))
}

/// A square tag. Used for risk and caveat, the only places colour appears besides
/// removal itself.
pub fn tag(
    theme: &Theme,
    content: impl Into<SharedString>,
    foreground: gpui::Rgba,
    background: gpui::Rgba,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .h(px(16.0))
        .px_1()
        .bg(background)
        .text_size(px(text::LABEL))
        .font_family(theme.font_ui.clone())
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(foreground)
        .child(content.into())
}

/// An empty state that teaches what the surface is for, rather than apologising.
pub fn empty_state(theme: &Theme, heading: &str, body: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .px_6()
        .py_6()
        .max_w(px(560.0))
        .child(
            div()
                .text_size(px(text::SUBTITLE))
                .font_family(theme.font_ui.clone())
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.ink)
                .child(heading.to_string()),
        )
        .child(
            div()
                .text_size(px(text::BODY))
                .font_family(theme.font_ui.clone())
                .text_color(theme.ink_muted)
                .child(body.to_string()),
        )
}

/// A failure, naming the problem and the recovery.
pub fn error_state(theme: &Theme, heading: &str, body: &str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .px_6()
        .py_6()
        .max_w(px(560.0))
        .border_l_1()
        .border_color(theme.accent)
        .child(
            div()
                .text_size(px(text::SUBTITLE))
                .font_family(theme.font_ui.clone())
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.accent)
                .child(heading.to_string()),
        )
        .child(
            div()
                .text_size(px(text::BODY))
                .font_family(theme.font_ui.clone())
                .text_color(theme.ink_muted)
                .child(body.to_string()),
        )
}

/// Work in progress: a rule-width bar that pulses, or a still bar when the user has
/// asked for less motion. Never a spinner.
pub fn loading_bar(theme: &Theme, id: &'static str) -> AnyElement {
    let bar = div().w_full().h(px(2.0)).bg(theme.accent);

    if theme.reduce_motion {
        return bar.into_any_element();
    }

    bar.with_animation(
        id,
        Animation::new(Duration::from_millis(1100)).repeat(),
        |element, delta| element.opacity(0.2 + 0.8 * delta),
    )
    .into_any_element()
}

/// A square checkbox. Not a pill, not a switch with a travelling knob — a box that
/// either has a mark in it or does not.
pub fn checkbox<F>(
    theme: &Theme,
    id: &str,
    checked: bool,
    caption: &str,
    on_click: F,
) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let (box_background, box_border, mark) = if checked {
        (theme.ink, theme.ink, theme.ink_inverse)
    } else {
        (rgba(0x00000000), theme.rule_strong, theme.ink_faint)
    };
    let hover = theme.surface_hover;
    let caption_color = if checked { theme.ink } else { theme.ink_muted };

    div()
        .id(id.to_string())
        .flex()
        .items_center()
        .gap_2()
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(on_click)
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(14.0))
                .h(px(14.0))
                .bg(box_background)
                .border_1()
                .border_color(box_border)
                .rounded(px(radius::CONTROL))
                .text_color(mark)
                .child(if checked {
                    icon(Icon::Check, 10.0).into_any_element()
                } else {
                    div().into_any_element()
                }),
        )
        .child(
            div()
                .text_size(px(text::SMALL))
                .font_family(theme.font_ui.clone())
                .text_color(caption_color)
                .child(caption.to_string()),
        )
        .into_any_element()
}

mod theme {
    pub const STAT_SIZE: f32 = 22.0;
}

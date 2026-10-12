//! The text field.
//!
//! GPUI ships no text input, so this is the real thing: a focus handle the app owns,
//! a visible focus edge, an I-beam cursor, and a clear affordance. Key handling is
//! delegated to the caller, which owns the string — the field never holds state.
//!
//! The face is monospace because what goes in it is always an identifier.

use crate::icons::{Icon, icon};
use crate::theme::{Theme, metrics, radius, space, text};
use gpui::{
    AnyElement, App, ClickEvent, CursorStyle, FocusHandle, IntoElement, KeyDownEvent, MouseButton,
    MouseDownEvent, Window, div, prelude::*, px,
};

pub fn text_field<F, G>(
    theme: &Theme,
    id: &str,
    value: &str,
    placeholder: &str,
    focus: &FocusHandle,
    on_key: F,
    on_clear: G,
) -> AnyElement
where
    F: Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
    G: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let empty = value.is_empty();
    let show_clear = !empty;
    let content_color = if empty {
        theme.muted_foreground
    } else {
        theme.foreground
    };
    let resting_edge = theme.input;
    let focus_edge = theme.ring;
    let hover_edge = theme.muted_foreground;
    let clear_color = theme.muted_foreground;
    // Hover closures must be `'static`, so every colour they need is copied out of
    // the theme before the closure is built.
    let clear_hover = theme.band(crate::theme::veil::BAND);
    let font = theme.font_mono.clone();
    let handle = focus.clone();

    div()
        .id(id.to_string())
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::SM))
        .h(px(metrics::CONTROL))
        .w_full()
        .px(px(space::SM))
        .bg(theme.background)
        .border_1()
        .border_color(resting_edge)
        .rounded(px(radius::CONTROL))
        .text_size(px(text::SMALL))
        .font_family(font)
        .text_color(content_color)
        .cursor(CursorStyle::IBeam)
        .track_focus(focus)
        // `track_focus` only registers that the field *can* hold focus. Something has
        // to grant it, and a click is the least surprising way.
        .on_mouse_down(MouseButton::Left, move |_event: &MouseDownEvent, window: &mut Window, cx: &mut App| {
            window.focus(&handle, cx);
        })
        .focus(move |style| style.border_color(focus_edge))
        .hover(move |style| style.border_color(hover_edge))
        .on_key_down(on_key)
        .child(icon(Icon::Search, crate::icons::INLINE, theme.muted_foreground))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .child(if empty {
                    placeholder.to_string()
                } else {
                    value.to_string()
                }),
        )
        .when(show_clear, |row| {
            row.child(
                div()
                    .id(format!("{id}-clear"))
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(16.0))
                    .h(px(16.0))
                    .flex_shrink_0()
                    .rounded(px(4.0))
                    .cursor(CursorStyle::PointingHand)
                    .hover(move |style| style.bg(clear_hover))
                    .on_click(on_clear)
                    .child(icon(Icon::Close, 12.0, clear_color)),
            )
        })
        .into_any_element()
}

//! The search field.
//!
//! GPUI ships no text input, so this is the real thing: a focus handle the app owns,
//! a visible focus rule, an I-beam cursor, and monospace content because what goes in
//! it is always an identifier. Key handling is delegated to the caller, which owns
//! the string.

use crate::icons::{Icon, icon};
use crate::theme::{Theme, layout, radius, text};
use gpui::{
    AnyElement, App, CursorStyle, FocusHandle, IntoElement, KeyDownEvent, MouseButton, MouseDownEvent,
    Window, div, prelude::*, px,
};

pub fn text_field<F>(
    theme: &Theme,
    id: &str,
    value: &str,
    placeholder: &str,
    focus: &FocusHandle,
    on_key: F,
) -> AnyElement
where
    F: Fn(&KeyDownEvent, &mut Window, &mut App) + 'static,
{
    let empty = value.is_empty();
    let content = if empty {
        placeholder.to_string()
    } else {
        value.to_string()
    };
    let content_color = if empty { theme.ink_faint } else { theme.ink };
    let focus_rule = theme.focus_ring;
    let idle_rule = theme.rule_strong;
    let hover_rule = theme.ink_faint;
    let font = theme.font_mono.clone();

    div()
        .id(id.to_string())
        .flex()
        .items_center()
        .gap_2()
        .h(px(layout::CONTROL))
        .px_3()
        .bg(theme.surface)
        .border_1()
        .border_color(idle_rule)
        .rounded(px(radius::CONTROL))
        .text_size(px(text::BODY))
        .font_family(font)
        .text_color(content_color)
        .cursor(CursorStyle::IBeam)
        .track_focus(focus)
        // `track_focus` only registers that the field *can* hold focus. Something has
        // to grant it, and a click is the least surprising way.
        .on_mouse_down(MouseButton::Left, {
            let handle = focus.clone();
            move |_event: &MouseDownEvent, window: &mut Window, cx: &mut App| {
                window.focus(&handle, cx);
            }
        })
        // The focus rule is the only 2px stroke in the interface, and it is the same
        // ink as everything else — on-brand rather than a borrowed blue.
        .focus(move |style| style.border_color(focus_rule))
        .hover(move |style| style.border_color(hover_rule))
        .on_key_down(on_key)
        .child(icon(Icon::Search, 14.0).text_color(theme.ink_faint))
        .child(content)
        .into_any_element()
}

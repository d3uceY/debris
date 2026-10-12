//! Buttons.
//!
//! Square, 2px radius, one hairline, no shadow. The four variants are the whole
//! vocabulary: `Default` for an ordinary action, `Primary` for the one action that
//! moves the task forward, `Danger` for removal, `Quiet` for a tertiary action that
//! only reveals itself on hover.

use crate::theme::{Theme, layout, radius, text};
use gpui::{
    AnyElement, App, ClickEvent, CursorStyle, FontWeight, IntoElement, SharedString, Window, div,
    prelude::*, px, rgba,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    Default,
    Primary,
    Danger,
    Quiet,
}

pub fn button<F>(
    theme: &Theme,
    id: impl Into<SharedString>,
    caption: impl Into<SharedString>,
    variant: ButtonVariant,
    enabled: bool,
    on_click: F,
) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let id = id.into();
    let caption = caption.into();
    let font = theme.font_ui.clone();

    // A disabled control stays legible instead of fading to invisibility: it keeps
    // its rule and its label, and loses only its weight.
    if !enabled {
        return div()
            .flex()
            .items_center()
            .justify_center()
            .h(px(layout::CONTROL))
            .px_3()
            .bg(rgba(0x00000000))
            .border_1()
            .border_color(theme.rule)
            .rounded(px(radius::CONTROL))
            .text_size(px(text::SMALL))
            .font_family(font)
            .text_color(theme.ink_faint)
            .child(caption)
            .into_any_element();
    }

    let (background, foreground, border, hover_background, hover_border) = match variant {
        ButtonVariant::Primary => (
            theme.ink,
            theme.ink_inverse,
            theme.ink,
            theme.ink_hover,
            theme.ink_hover,
        ),
        ButtonVariant::Danger => (
            theme.accent,
            theme.accent_text,
            theme.accent,
            theme.accent_hover,
            theme.accent_hover,
        ),
        ButtonVariant::Default => (
            rgba(0x00000000),
            theme.ink,
            theme.rule_strong,
            theme.surface_hover,
            theme.ink,
        ),
        ButtonVariant::Quiet => (
            rgba(0x00000000),
            theme.ink_muted,
            rgba(0x00000000),
            theme.surface_hover,
            rgba(0x00000000),
        ),
    };

    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .h(px(layout::CONTROL))
        .px_3()
        .bg(background)
        .border_1()
        .border_color(border)
        .rounded(px(radius::CONTROL))
        .text_size(px(text::SMALL))
        .font_family(font)
        .font_weight(FontWeight::MEDIUM)
        .text_color(foreground)
        .cursor(CursorStyle::PointingHand)
        .hover(move |style| {
            style
                .bg(hover_background)
                .border_color(hover_border)
        })
        .on_click(on_click)
        .child(caption)
        .into_any_element()
}


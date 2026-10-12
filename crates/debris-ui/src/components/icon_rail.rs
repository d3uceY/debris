//! The icon rail — the vertical surface switcher on the far left.
//!
//! Faithful to the reference app's Slack-style rail, and for the same reason: the
//! surfaces of an app are a small, permanent, closed set, and giving them 48px of
//! permanent width is cheaper than giving them a row of text that competes with the
//! content. A rail button has no label; its tooltip is its label.
//!
//! The rail's bottom slot is reserved for the one global preference that changes
//! how everything above it reads (here: appearance).

use crate::components::tooltip::tip;
use crate::icons::{Icon, RAIL, icon};
use crate::theme::{Theme, metrics, radius, space};
use gpui::{AnyElement, ClickEvent, IntoElement, ParentElement, SharedString, Styled, Window, div, prelude::*, px};

/// One rail button: a 34px square with a mark in it.
pub fn rail_item<F>(
    theme: &Theme,
    id: impl Into<gpui::ElementId>,
    mark: Icon,
    tooltip: impl Into<SharedString>,
    active: bool,
    on_click: F,
) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    let fill = if active {
        theme.sidebar_accent
    } else {
        gpui::rgba(0x00000000)
    };
    let hover = theme.sidebar_accent.opacity(0.6);
    let color = if active {
        theme.foreground
    } else {
        theme.muted_foreground
    };

    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(metrics::RAIL_ITEM))
        .h(px(metrics::RAIL_ITEM))
        .flex_shrink_0()
        .rounded(px(radius::RAIL))
        .bg(fill)
        .text_color(color)
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(on_click)
        .tooltip(tip(tooltip))
        .child(icon(mark, RAIL, color))
        .into_any_element()
}

/// A short rule between groups of rail buttons.
pub fn rail_divider(theme: &Theme) -> AnyElement {
    div()
        .w(px(20.0))
        .h(px(8.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .child(div().w_full().h(px(1.0)).bg(theme.border))
        .into_any_element()
}

/// The rail's own frame.
pub fn rail_shell(theme: &Theme) -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        // A gap, not a margin: unselected rail buttons are invisible, so the gap is
        // what keeps a *selected* one from looking like it grew out of its neighbour.
        .gap(px(space::XS))
        .w(px(metrics::ICON_RAIL))
        .h_full()
        .flex_shrink_0()
        .py(px(space::SM))
        .bg(theme.sidebar)
        .border_r_1()
        .border_color(theme.sidebar_border)
}

/// The slot at the bottom of the rail. `space_between` keeps the top group pinned
/// to the top and this one pinned to the bottom without a spacer element.
pub fn rail_spacer() -> AnyElement {
    div().flex_1().into_any_element()
}

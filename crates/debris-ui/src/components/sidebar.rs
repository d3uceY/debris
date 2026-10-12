//! The left column.
//!
//! In the reference shell this is the object browser. In debris it is the **removal
//! queue**, and it is deliberately not screen-dependent: staging is the one piece of
//! state that outlives the surface you are standing on, so hiding it behind a tab
//! would mean a preset loaded on the Presets screen silently scrolls out of sight.
//!
//! The column is three parts — a header that names it and counts it, a body that
//! scrolls, and a footer that concludes it. The footer is part of the column rather
//! than a dialog, because removal is a decision worth keeping on screen rather than
//! worth interrupting for.

use crate::components::eyebrow;
use crate::theme::{Theme, metrics, space, text, veil};
use gpui::{AnyElement, Div, IntoElement, ParentElement, SharedString, Styled, div, prelude::*, px};

/// The column frame: fixed width, its own surface, and one edge.
pub fn side_column(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .w(px(metrics::SIDEBAR))
        .h_full()
        .flex_shrink_0()
        .min_h_0()
        .bg(theme.sidebar)
        .border_r_1()
        .border_color(theme.sidebar_border)
}

/// The column's title strip. Same height as a panel header, so the two regions'
/// bands line up across the app.
pub fn column_header(
    theme: &Theme,
    label: impl Into<SharedString>,
    count: Option<usize>,
    actions: Option<AnyElement>,
) -> AnyElement {
    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap(px(space::SM))
        .h(px(metrics::PANEL_HEADER))
        .w_full()
        .flex_shrink_0()
        .px(px(space::MD))
        .border_b_1()
        .border_color(theme.border.opacity(veil::HAIRLINE))
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(space::SM))
                .min_w_0()
                .child(eyebrow(theme, label.into()))
                .when_some(count, |el, count| {
                    el.child(
                        div()
                            .font_family(theme.font_mono.clone())
                            .text_size(px(text::LABEL))
                            .text_color(theme.muted_foreground)
                            .child(count.to_string()),
                    )
                }),
        );

    if let Some(actions) = actions {
        row = row.child(actions);
    }
    row.into_any_element()
}

/// A left column's scroll region.
pub fn column_body() -> Div {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .w_full()
        .overflow_hidden()
}

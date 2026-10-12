//! Panel chrome — the strips that wrap a region of content.
//!
//! A boxed region in this interface is three horizontal bands and a body:
//!
//! ```text
//!   ┌── panel_header ── 34px ── title + subtitle + actions ────────┐
//!   ├── toolbar ─────── 30px ── controls ──────────────────────────┤
//!   │  body              flex_1, min_h_0, scrolls or virtualizes    │
//!   ├── panel_footer ────────── the one action that concludes it ──┤
//!   └───────────────────────────────────────────────────────────────┘
//! ```
//!
//! Every band carries the same tint (`muted` at [`veil::BAND`]) and the same
//! hairline. The bands are what make a region read as a region — there is no card,
//! no shadow, and no radius around the body.

use crate::theme::{Theme, metrics, space, text, veil};
use gpui::{AnyElement, Div, FontWeight, IntoElement, ParentElement, SharedString, Styled, div, px};

/// The title strip on a boxed panel.
///
/// The title is paired with a one-line subtitle, always. A panel that can only say
/// its own name has not said enough: the subtitle is where the count, the mode, or
/// the scope lives.
pub fn panel_header(
    theme: &Theme,
    title: impl Into<SharedString>,
    subtitle: impl Into<SharedString>,
    actions: Option<AnyElement>,
) -> AnyElement {
    let font = theme.font_ui.clone();
    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::SM))
        .h(px(metrics::PANEL_HEADER))
        .w_full()
        .flex_shrink_0()
        .px(px(space::MD))
        .bg(theme.band(veil::BAND_QUIET))
        .border_b_1()
        .border_color(theme.border)
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(0.0))
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .truncate()
                        .font_family(font.clone())
                        .text_size(px(text::SUBTITLE))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(title.into()),
                )
                .child(
                    div()
                        .truncate()
                        .font_family(font)
                        .text_size(px(text::LABEL))
                        .text_color(theme.muted_foreground)
                        .child(subtitle.into()),
                ),
        );

    if let Some(actions) = actions {
        row = row.child(actions);
    }
    row.into_any_element()
}

/// A strip of controls between a header and its content.
pub fn toolbar(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::SM))
        .h(px(metrics::TOOLBAR))
        .w_full()
        .flex_shrink_0()
        .px(px(space::MD))
        .bg(theme.band(veil::BAND_QUIET))
        .border_b_1()
        .border_color(theme.border)
}

/// The strip that concludes a region: the summary of what the action will do, and
/// the action.
pub fn panel_footer(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(space::SM))
        .w_full()
        .flex_shrink_0()
        .p(px(space::MD))
        .bg(theme.band(veil::BAND_QUIET))
        .border_t_1()
        .border_color(theme.border)
}

/// The body of a panel: it fills the space between the bands and is the only thing
/// that scrolls.
pub fn panel_body() -> Div {
    div().flex().flex_col().flex_1().min_h_0().w_full().overflow_hidden()
}

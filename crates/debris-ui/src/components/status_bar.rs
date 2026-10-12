//! The bottom edge.
//!
//! One line, always in the same place. The original tool reported results in modal
//! dialogs; a fixed status line means the answer to "what just happened" is never
//! something the user has to dismiss or remember.

use crate::icons::{Icon, icon};
use crate::snapshot::{Status, StatusTone};
use crate::theme::{Theme, layout, text};
use gpui::{IntoElement, div, prelude::*, px};

pub fn status_bar(
    theme: &Theme,
    status: &Status,
    elevated: Option<bool>,
    reduce_motion: bool,
) -> impl IntoElement {
    let (dot, text_color) = match status.tone() {
        StatusTone::Neutral => (theme.ink_faint, theme.ink_muted),
        StatusTone::Good => (theme.success, theme.ink),
        StatusTone::Caution => (theme.warning, theme.ink),
        StatusTone::Bad => (theme.accent, theme.accent),
    };

    let (elevation_label, elevation_color) = match elevated {
        Some(true) => ("Administrator", theme.ink),
        Some(false) => ("Standard user", theme.ink_faint),
        None => ("Elevation not checked", theme.ink_faint),
    };

    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .h(px(layout::STATUS_BAR))
        .px_4()
        .flex_shrink_0()
        .bg(theme.surface)
        .border_t_1()
        .border_color(theme.rule)
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(div().w(px(6.0)).h(px(6.0)).bg(dot))
                .child(
                    div()
                        .font_family(theme.font_ui.clone())
                        .text_size(px(text::SMALL))
                        .text_color(text_color)
                        .truncate()
                        .child(status.text().to_string()),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .flex_shrink_0()
                .text_color(elevation_color)
                .child(icon(Icon::Shield, 12.0))
                .child(
                    div()
                        .font_family(theme.font_ui.clone())
                        .text_size(px(text::LABEL))
                        .child(elevation_label.to_string()),
                ),
        )
        .child(
            div()
                .flex_shrink_0()
                .font_family(theme.font_ui.clone())
                .text_size(px(text::LABEL))
                .text_color(theme.ink_faint)
                .child(if reduce_motion {
                    "Motion: reduced".to_string()
                } else {
                    "Motion: full".to_string()
                }),
        )
}

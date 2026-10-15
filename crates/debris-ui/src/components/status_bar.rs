//! The bottom edge.
//!
//! One line, always in the same place. The original tool reported results in modal
//! dialogs; a fixed status rail means the answer to "what just happened" is never
//! something the user has to dismiss or remember.
//!
//! It is built from **segments**, not messages: a label and a value, separated by a
//! hairline. The left segment is the only one that changes width, so the right side
//! never moves.

use crate::components::divider;
use crate::components::pulse_dot;
use crate::icons::{Icon, icon};
use crate::snapshot::{Status, StatusTone};
use crate::theme::{Theme, metrics, space, text};
use gpui::{AnyElement, IntoElement, ParentElement, Rgba, SharedString, div, prelude::*, px};

/// The build this window is.
///
/// Every crate in the workspace takes its version from `[workspace.package]`, so this
/// is the app's version too, and it is the same number the release tag carries: the
/// release workflow stamps the tag's CalVer into the manifest before it builds, and a
/// local build is simply the version checked into that manifest. That is deliberate —
/// *"which build is this?"* has one answer, and it is the tag.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// One label/value pair on the rail.
pub fn segment(theme: &Theme, label: &str, value: impl Into<SharedString>, dot: Option<Rgba>) -> AnyElement {
    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::XS))
        .flex_shrink_0();

    if let Some(color) = dot {
        row = row.child(div().w(px(5.0)).h(px(5.0)).flex_shrink_0().rounded_full().bg(color));
    }
    row.child(
        div()
            .text_size(px(text::LABEL))
            .text_color(theme.muted_foreground)
            .child(label.to_string()),
    )
    .child(
        div()
            .text_size(px(text::LABEL))
            .font_family(theme.font_mono.clone())
            .text_color(theme.foreground)
            .child(value.into()),
    )
    .into_any_element()
}

/// The rail itself.
///
/// `left` is the changing message; `right` is the fixed set of state segments. The
/// message is truncated rather than wrapped, so the rail keeps its 26px.
pub fn status_bar(
    theme: &Theme,
    status: &Status,
    elevated: Option<bool>,
    busy: bool,
) -> AnyElement {
    let (dot, message_color) = match status.tone() {
        StatusTone::Neutral => (theme.muted_foreground, theme.foreground),
        StatusTone::Caution => (theme.warning, theme.foreground),
        StatusTone::Bad => (theme.danger, theme.danger),
    };

    let (elevation_label, elevation_color, elevation_mark) = match elevated {
        Some(true) => ("Administrator", theme.foreground, Icon::Shield),
        Some(false) => ("Standard user", theme.muted_foreground, Icon::Shield),
        None => ("Elevation not checked", theme.muted_foreground, Icon::Shield),
    };

    let dot_element: AnyElement = if busy {
        pulse_dot("status-pulse", theme.primary)
    } else {
        div().w(px(6.0)).h(px(6.0)).flex_shrink_0().rounded_full().bg(dot).into_any_element()
    };

    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::MD))
        .h(px(metrics::STATUS_BAR))
        .w_full()
        .px(px(space::MD))
        .flex_shrink_0()
        .bg(theme.background)
        .border_t_1()
        .border_color(theme.border)
        // — left: what just happened —
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(space::SM))
                .flex_1()
                .min_w_0()
                .child(dot_element)
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .font_family(theme.font_ui.clone())
                        .text_size(px(text::SMALL))
                        .text_color(message_color)
                        .child(status.text().to_string()),
                ),
        )
        // — right: the state of the machine, fixed —
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(space::MD))
                .flex_shrink_0()
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(space::XS))
                        .flex_shrink_0()
                        .child(icon(elevation_mark, 12.0, elevation_color))
                        .child(
                            div()
                                .text_size(px(text::LABEL))
                                .child(elevation_label.to_string()),
                        ),
                )
                // The build identity sits hard right, past the state segments: it is
                // the one thing on the rail that never changes while the app is open.
                .child(divider(theme))
                .child(
                    div()
                        .font_family(theme.font_mono.clone())
                        .text_size(px(text::LABEL))
                        .text_color(theme.muted_foreground)
                        .child(format!("v{VERSION}")),
                ),
        )
        .into_any_element()
}

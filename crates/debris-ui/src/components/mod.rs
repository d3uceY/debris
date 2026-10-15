//! Reusable pieces of the interface.
//!
//! Every component takes `&Theme` rather than reaching for a global, so a component
//! can be rendered against either appearance in a test or a preview. Nothing here
//! holds state; behaviour arrives as a closure.
//!
//! # The vocabulary
//!
//! There are seven shapes in this interface and nothing else:
//!
//! * a **band** — a full-width tinted strip that separates two regions,
//! * a **rule** — hairline separation inside a region,
//! * a **row** — the unit of every list,
//! * a **chip** — a badge, a keycap, a count pill,
//! * a **panel header** — a title strip on a boxed region,
//! * an **empty state** — a tile, a title, and a sentence,
//! * a **control** — a button, a field, a switch.
//!
//! If a screen needs an eighth shape, that is a design decision, not an
//! implementation detail.

pub mod button;
pub mod icon_rail;
pub mod panel;
pub mod rows;
pub mod sidebar;
pub mod status_bar;
pub mod text_field;
pub mod tooltip;

use crate::icons::{Icon, icon};
use crate::theme::{Theme, radius, space, text, veil};
use gpui::{
    Animation, AnimationExt, AnyElement, App, ClickEvent, Div, FontWeight, IntoElement,
    SharedString, Window, div, prelude::*, px,
};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Flex shortcuts
// ---------------------------------------------------------------------------

/// A row that centres its children on the cross axis. GPUI's default direction is
/// already a row; this adds the alignment every shell row wants.
pub fn h_flex() -> Div {
    div().flex().flex_row().items_center()
}

/// A column.
pub fn v_flex() -> Div {
    div().flex().flex_col()
}

// ---------------------------------------------------------------------------
// Structure
// ---------------------------------------------------------------------------

/// A vertical hairline, for separating items in a horizontal band.
pub fn divider(theme: &Theme) -> impl IntoElement {
    div()
        .w(px(1.0))
        .h(px(14.0))
        .flex_shrink_0()
        .bg(theme.border)
}

// ---------------------------------------------------------------------------
// Type
// ---------------------------------------------------------------------------

/// A small, bold, muted section label. It names a region; it is never a heading
/// stacked above a heading.
pub fn eyebrow(theme: &Theme, content: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(text::LABEL))
        .font_family(theme.font_ui.clone())
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme.muted_foreground)
        .child(content.into())
}

/// A section label with its item count pushed to the far edge — the standard way a
/// region announces how much is in it.
pub fn section_eyebrow(
    theme: &Theme,
    label: impl Into<SharedString>,
    count: Option<usize>,
) -> Div {
    let mut row = h_flex()
        .w_full()
        .flex_shrink_0()
        .justify_between()
        .gap(px(space::SM))
        .px(px(space::MD))
        .py(px(space::XS + 2.0))
        .child(eyebrow(theme, label));

    if let Some(count) = count {
        row = row.child(
            div()
                .font_family(theme.font_mono.clone())
                .text_size(px(text::LABEL))
                .text_color(theme.muted_foreground)
                .child(count.to_string()),
        );
    }
    row
}

// ---------------------------------------------------------------------------
// Chips
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    /// Emphasis without colour: the chip is a shade of the page.
    Neutral,
    /// A statement of fact that should not pull the eye.
    Quiet,
    Danger,
    Warning,
    Success,
}

/// The foreground and fill for a tone, from tokens only.
pub fn tone_colors(theme: &Theme, tone: Tone) -> (gpui::Rgba, Option<gpui::Rgba>, Option<gpui::Rgba>) {
    match tone {
        Tone::Neutral => (theme.foreground, Some(theme.muted.opacity(veil::CHIP)), None),
        Tone::Quiet => (theme.muted_foreground, None, Some(theme.border)),
        Tone::Danger => (theme.danger, Some(theme.danger_soft), None),
        Tone::Warning => (theme.warning, Some(theme.warning_soft), None),
        Tone::Success => (theme.success, Some(theme.success_soft), None),
    }
}

/// A small chip: a badge, a category, a state.
pub fn chip(theme: &Theme, content: impl Into<SharedString>, tone: Tone) -> Div {
    chip_with(theme, None, content, tone)
}

/// A chip with a leading mark, for a state that reads better as icon + word.
pub fn chip_with(
    theme: &Theme,
    mark: Option<Icon>,
    content: impl Into<SharedString>,
    tone: Tone,
) -> Div {
    let (foreground, fill, border) = tone_colors(theme, tone);
    let element = h_flex()
        .flex_shrink_0()
        .h(px(18.0))
        .gap(px(space::XS))
        .px(px(6.0))
        .rounded(px(radius::CHIP))
        .text_size(px(text::LABEL))
        .font_family(theme.font_ui.clone())
        .font_weight(FontWeight::MEDIUM)
        .text_color(foreground);

    // Children are appended in build order, so the mark has to be attached before
    // the label rather than after: build the shell, add fill/border, then the two
    // children in reading order.
    let element = match fill {
        Some(fill) => element.bg(fill),
        None => element,
    };
    let element = match border {
        Some(border) => element.border_1().border_color(border),
        None => element,
    };
    let element = match mark {
        Some(mark) => element.child(icon(mark, 11.0, foreground)),
        None => element,
    };
    element.child(content.into())
}

/// A keycap. Mono, hairline, barely raised — it is an instruction, not a button.
pub fn kbd(theme: &Theme, stroke: impl Into<SharedString>) -> Div {
    h_flex()
        .flex_shrink_0()
        .h(px(17.0))
        .justify_center()
        .px(px(5.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(theme.border)
        .bg(theme.band(veil::BAND))
        .font_family(theme.font_mono.clone())
        .text_size(px(text::LABEL))
        .text_color(theme.muted_foreground)
        .child(stroke.into())
}

// ---------------------------------------------------------------------------
// Controls
// ---------------------------------------------------------------------------

/// A switch. The knob travels by changing the row's justification rather than by
/// transforming it, because GPUI has no element transforms — so the state change is
/// instant and the *colour* carries the feedback.
pub fn switch<F>(
    theme: &Theme,
    id: &str,
    on: bool,
    caption: &str,
    on_click: F,
) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let track = if on { theme.primary } else { theme.input };
    let knob = if on {
        theme.primary_foreground
    } else {
        theme.background
    };
    let caption_color = if on {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    let hover = theme.accent.opacity(0.5);
    let font = theme.font_ui.clone();

    h_flex()
        .id(id.to_string())
        .gap(px(space::SM))
        .py(px(2.0))
        .cursor_pointer()
        .rounded(px(radius::CHIP))
        .hover(move |style| style.bg(hover))
        .on_click(on_click)
        .child(
            div()
                .flex()
                .items_center()
                .w(px(28.0))
                .h(px(16.0))
                .flex_shrink_0()
                .p(px(2.0))
                .rounded_full()
                .bg(track)
                .when(on, |el| el.justify_end())
                .when(!on, |el| el.justify_start())
                .child(div().w(px(12.0)).h(px(12.0)).flex_shrink_0().rounded_full().bg(knob)),
        )
        .child(
            div()
                .text_size(px(text::SMALL))
                .font_family(font)
                .text_color(caption_color)
                .child(caption.to_string()),
        )
        .into_any_element()
}

// ---------------------------------------------------------------------------
// States
// ---------------------------------------------------------------------------

/// An empty state that teaches what the surface is for, rather than apologising.
pub fn empty_state(
    theme: &Theme,
    mark: Icon,
    heading: &str,
    body: &str,
    action: Option<AnyElement>,
) -> AnyElement {
    let mut column = v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap(px(space::MD))
        .p(px(space::XL))
        .child(
            // The tile gives an otherwise empty region a centre of gravity.
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(42.0))
                .h(px(42.0))
                .flex_shrink_0()
                .rounded(px(radius::TILE))
                .border_1()
                .border_color(theme.border)
                .bg(theme.band(veil::CHIP))
                .child(icon(mark, crate::icons::TILE, theme.muted_foreground)),
        )
        .child(state_copy(theme, heading, body, 360.0));

    if let Some(action) = action {
        column = column.child(action);
    }
    column.into_any_element()
}

/// The heading-and-body block both states share: a title over a wrapped sentence.
fn state_copy(theme: &Theme, heading: &str, body: &str, max_w: f32) -> Div {
    v_flex()
        .items_center()
        .gap(px(3.0))
        .child(
            div()
                .text_size(px(text::SUBTITLE))
                .font_family(theme.font_ui.clone())
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(heading.to_string()),
        )
        .child(
            div()
                .max_w(px(max_w))
                .text_size(px(text::SMALL))
                .font_family(theme.font_ui.clone())
                .text_color(theme.muted_foreground)
                .text_center()
                .child(body.to_string()),
        )
}

/// A failure, naming the problem and the recovery.
pub fn error_state(theme: &Theme, heading: &str, body: &str) -> AnyElement {
    v_flex()
        .size_full()
        .items_center()
        .justify_center()
        .gap(px(space::MD))
        .p(px(space::XL))
        .child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(42.0))
                .h(px(42.0))
                .flex_shrink_0()
                .rounded(px(radius::TILE))
                .border_1()
                .border_color(theme.danger.opacity(0.35))
                .bg(theme.danger_soft)
                .child(icon(Icon::Alert, crate::icons::TILE, theme.danger)),
        )
        .child(state_copy(theme, heading, body, 400.0))
        .into_any_element()
}

/// A lightweight muted hint for a pane with nothing in it. Use this instead of
/// [`empty_state`] where a 42px tile and two lines would be too heavy.
pub fn pane_hint(theme: &Theme, message: impl Into<SharedString>) -> AnyElement {
    v_flex()
        .flex_1()
        .items_center()
        .justify_center()
        .p(px(space::MD))
        .child(
            div()
                .text_size(px(text::SMALL))
                .font_family(theme.font_ui.clone())
                .text_color(theme.muted_foreground)
                .text_center()
                .child(message.into()),
        )
        .into_any_element()
}

/// Work in progress: a rule-width bar that pulses, under the header. Never a spinner
/// in the middle of a layout that is about to be replaced by content.
///
/// It always animates — there is no preference that turns it off (see `DESIGN.md`).
/// The id is not decoration: an animation is keyed, and an unkeyed one restarts on
/// every frame.
pub fn loading_bar(theme: &Theme, id: &'static str) -> AnyElement {
    div()
        .w_full()
        .h(px(2.0))
        .flex_shrink_0()
        .bg(theme.primary)
        .with_animation(
            id,
            Animation::new(Duration::from_millis(1100)).repeat(),
            |element, delta| element.opacity(0.2 + 0.8 * delta),
        )
        .into_any_element()
}

/// A small pulsing dot, for the status rail's live segment. Like the loading bar it
/// always pulses; the animation is state feedback, not an option.
pub fn pulse_dot(id: &'static str, color: gpui::Rgba) -> AnyElement {
    div()
        .w(px(6.0))
        .h(px(6.0))
        .flex_shrink_0()
        .rounded_full()
        .bg(color)
        .with_animation(
            id,
            Animation::new(Duration::from_millis(900)).repeat(),
            |element, delta| element.opacity(0.35 + 0.65 * delta),
        )
        .into_any_element()
}

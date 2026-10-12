//! Rows.
//!
//! # The row anatomy
//!
//! Every list in this interface is built from the same four slots, in the same
//! order, so the eye only has to learn one rhythm:
//!
//! ```text
//!   ┌ lead ┐ ┌── flex_1 ──────┐ ┌ fixed ┐ ┌ fixed ┐ ┌ trailing ┐
//!   │ mark │ │ name / title   │ │ meta  │ │ meta  │ │ chip/act │
//!   └──────┘ └────────────────┘ └───────┘ └───────┘ └──────────┘
//! ```
//!
//! * **lead** is a fixed-width slot. A row without a mark still reserves it, so
//!   labels align down the column whether or not every row has an icon.
//! * **name** is the only flexible slot. It carries `min_w_0` and `truncate`, so it
//!   is always the thing that gives way.
//! * **meta** slots are fixed widths, right-sized to their content with room to
//!   spare. Fixed is what makes a stack of rows read as a table.
//! * **trailing** holds a chip or an action, and is `flex_shrink_0`.
//!
//! A row declares its own rule (`border_b`) rather than sitting inside a bordered
//! container. A list is a stack of hairlines, not a card with padding.

use crate::components::{Tone, chip, chip_with};
use crate::icons::{Icon, icon};
use crate::theme::{Theme, metrics, radius, space, text, veil};
use debris_core::{AppxPackage, Caveat, Category, Preset, Risk, ScriptMeta};
use gpui::{
    AnyElement, App, ClickEvent, FontWeight, IntoElement, Rgba, SharedString, Window, div,
    prelude::*, px, rgba,
};

/// Fixed widths shared by a table's header and its rows. One definition means the
/// columns cannot drift apart.
pub mod column {
    /// The lead slot: the mark that says what a row *is*, or whether it is staged.
    pub const LEAD: f32 = 22.0;
    pub const PUBLISHER: f32 = 168.0;
    pub const VERSION: f32 = 96.0;
    /// The leading inset every row and every column header shares.
    pub const INSET: f32 = 8.0;
    /// The trailing inset. Slightly wider, so the last column is not flush to the
    /// scrollbar.
    pub const TRAILING_INSET: f32 = 12.0;
}

/// The mark in a row's lead slot, at a size that does not fight the label.
pub const LEAD_ICON: f32 = 14.0;

// ---------------------------------------------------------------------------
// Chips derived from domain values — the only place colour is allowed to appear
// ---------------------------------------------------------------------------

/// A caveat as a chip. `IRREPLACEABLE` is the one that occasionally stops someone,
/// so it gets the removal colour; `SYSTEM` is a statement of fact and gets caution.
pub fn caveat_chip(theme: &Theme, caveat: Caveat) -> AnyElement {
    let (mark, tone) = match caveat {
        Caveat::SystemCritical => (Icon::Cpu, Tone::Warning),
        Caveat::Store => (Icon::Store, Tone::Danger),
    };
    chip_with(theme, Some(mark), caveat.label(), tone).into_any_element()
}

/// A caveat's sentence, for an inspector.
pub fn caveat_detail(caveat: Caveat) -> &'static str {
    caveat.detail()
}

pub fn risk_chip(theme: &Theme, risk: Risk) -> AnyElement {
    let tone = match risk {
        Risk::ReadOnly => Tone::Quiet,
        Risk::Reversible => Tone::Neutral,
        Risk::Destructive => Tone::Danger,
    };
    chip(theme, risk.label(), tone).into_any_element()
}

pub fn category_chip(theme: &Theme, category: Category) -> AnyElement {
    chip(theme, category.label(), Tone::Quiet).into_any_element()
}

// ---------------------------------------------------------------------------
// Selection paint
// ---------------------------------------------------------------------------

/// The three background states a row can be in, resolved from tokens.
fn row_paint(theme: &Theme, staged: bool, selected: bool) -> (Rgba, Rgba, Rgba) {
    // (resting fill, hover fill, lead marker)
    if staged {
        let wash = theme.danger_wash();
        return (wash, theme.danger.opacity(0.14), theme.danger);
    }
    if selected {
        return (theme.accent, theme.accent, theme.foreground);
    }
    (rgba(0x00000000), theme.band(veil::BAND), rgba(0x00000000))
}

// ---------------------------------------------------------------------------
// Package rows
// ---------------------------------------------------------------------------

/// The mark that occupies a package row's lead slot.
///
/// A staged package shows a check in the removal colour, because in the queue the
/// only question worth answering is "is this one actually selected".
fn package_lead(theme: &Theme, staged: bool, selected: bool) -> AnyElement {
    let color = if staged {
        theme.danger
    } else if selected {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    let mark = if staged { Icon::Check } else { Icon::Package };

    div()
        .flex()
        .items_center()
        .justify_center()
        .w(px(column::LEAD))
        .flex_shrink_0()
        .child(icon(mark, LEAD_ICON, color))
        .into_any_element()
}

/// A package in the inventory table.
///
/// The inventory and the queue are two different *views* of the same row: this one
/// is a table row with columns, [`queue_row`] is the compact column form. Both draw
/// the same mark for a staged package, so a package that leaves the table is
/// visibly the same thing that arrived in the column.
pub fn package_row<F>(
    theme: &Theme,
    id: (&'static str, usize),
    package: &AppxPackage,
    staged: bool,
    selected: bool,
    on_click: F,
) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let (background, hover, _) = row_paint(theme, staged, selected);
    let name_color = if staged { theme.danger } else { theme.foreground };
    let font_mono = theme.font_mono.clone();
    let font_ui = theme.font_ui.clone();
    let caveat = package_caveat(package, staged);

    let row = div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        // `w_full` is load-bearing: without it the row is content-sized, the name
        // cell's `flex_1` has nothing to grow into, and every fixed column drifts
        // left or right depending on how long the name happens to be.
        .w_full()
        .h(px(metrics::ROW))
        .flex_shrink_0()
        .pl(px(column::INSET))
        .pr(px(column::TRAILING_INSET))
        .gap(px(space::SM))
        .bg(background)
        .border_b_1()
        .border_color(theme.border.opacity(veil::ROW_RULE))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(on_click)
        .child(package_lead(theme, staged, selected))
        // The name cell holds the caveat chip too. A separate trailing "Note" column
        // was the obvious shape and the wrong one: with system components hidden —
        // the default — that column is empty in every row, which is a column that has
        // no job. Inline, the chip sits where the eye already is, and it costs the
        // name only as much width as the caveat actually needs.
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(space::SM))
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .font_family(font_mono)
                        .text_size(px(text::SMALL))
                        .text_color(name_color)
                        .child(package.name.clone()),
                )
                .when_some(caveat, |cell, caveat| {
                    cell.child(caveat_chip(theme, caveat))
                }),
        )
        .child(
            div()
                .w(px(column::PUBLISHER))
                .flex_shrink_0()
                .truncate()
                .font_family(font_ui)
                .text_size(px(text::SMALL))
                .text_color(theme.muted_foreground)
                .child(package.publisher_short().to_string()),
        )
        .child(
            div()
                .w(px(column::VERSION))
                .flex_shrink_0()
                .truncate()
                .font_family(theme.font_mono.clone())
                .text_size(px(text::LABEL))
                .text_color(theme.muted_foreground)
                .child(package.version.clone()),
        );

    row.into_any_element()
}

/// The table's column header. Uses the same widths as [`package_row`], so the two
/// cannot drift apart.
pub fn package_columns(theme: &Theme) -> AnyElement {
    let label = |content: &str, width: Option<f32>| -> AnyElement {
        let mut el = div()
            .text_size(px(text::LABEL))
            .font_family(theme.font_ui.clone())
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(theme.muted_foreground)
            .child(content.to_string());
        if let Some(width) = width {
            el = el.w(px(width)).flex_shrink_0();
        } else {
            el = el.flex_1().min_w_0();
        }
        el.into_any_element()
    };

    div()
        .flex()
        .flex_row()
        .items_center()
        .h(px(metrics::COLUMN_HEADER))
        .flex_shrink_0()
        .pl(px(column::INSET))
        .pr(px(column::TRAILING_INSET))
        .gap(px(space::SM))
        .bg(theme.band(veil::BAND_QUIET))
        .border_b_1()
        .border_color(theme.border)
        .child(div().w(px(column::LEAD)).flex_shrink_0())
        .child(label("Package", None))
        .child(label("Publisher", Some(column::PUBLISHER)))
        .child(label("Version", Some(column::VERSION)))
        .into_any_element()
}

/// The compact form of a package row, for the removal queue's 304px column.
///
/// Wider metadata has nowhere to go here, so only the two things that change a
/// decision survive: the name, and whether the package is system-critical.
pub fn queue_row<F>(
    theme: &Theme,
    id: (&'static str, usize),
    package: &AppxPackage,
    on_click: F,
) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let wash = theme.danger_wash();
    let hover = theme.danger.opacity(0.14);
    let caveat = package_caveat(package, true);
    let mut row = div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .w_full()
        .h(px(metrics::ROW))
        .flex_shrink_0()
        .pl(px(column::INSET))
        .pr(px(space::SM))
        .gap(px(space::SM))
        .bg(wash)
        .border_b_1()
        .border_color(theme.border.opacity(veil::ROW_RULE))
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(on_click)
        .child(package_lead(theme, true, false))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_family(theme.font_mono.clone())
                .text_size(px(text::SMALL))
                .text_color(theme.foreground)
                .child(package.name.clone()),
        );

    if let Some(caveat) = caveat {
        let color = match caveat {
            Caveat::SystemCritical => theme.warning,
            Caveat::Store => theme.danger,
        };
        row = row.child(icon(
            match caveat {
                Caveat::SystemCritical => Icon::Cpu,
                Caveat::Store => Icon::Store,
            },
            12.0,
            color,
        ));
    }

    row.into_any_element()
}

/// The caveat is only worth showing where it changes a decision: on a row the user
/// has already staged, or on a system component they chose to reveal.
pub fn package_caveat(package: &AppxPackage, staged: bool) -> Option<Caveat> {
    if package.name.eq_ignore_ascii_case("Microsoft.WindowsStore") {
        return Some(Caveat::Store);
    }
    if package.is_system {
        return Some(Caveat::SystemCritical);
    }
    let _ = staged;
    None
}

// ---------------------------------------------------------------------------
// Script rows
// ---------------------------------------------------------------------------

/// A script in the library. Taller than a package row, because it carries prose and
/// the prose is the point: the user has to read what it does before running it.
pub fn script_row(theme: &Theme, script: &ScriptMeta, actions: AnyElement) -> AnyElement {
    let risk = script.risk;
    let font_ui = theme.font_ui.clone();
    let font_mono = theme.font_mono.clone();
    let muted = theme.muted_foreground;

    let mut body = div()
        .flex()
        .flex_col()
        .w_full()
        .gap(px(space::XS))
        .px(px(space::MD))
        .py(px(space::SM + 2.0))
        .border_b_1()
        .border_color(theme.border.opacity(veil::ROW_RULE));

    body = body.child(
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(space::SM))
            .child(icon(Icon::Terminal, LEAD_ICON, muted))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_family(font_ui.clone())
                    .text_size(px(text::SUBTITLE))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.foreground)
                    .child(script.title.clone()),
            )
            .child(risk_chip(theme, risk))
            .when(script.requires_admin, |row| {
                row.child(chip_with(theme, Some(Icon::Shield), "admin", Tone::Quiet))
            })
            .child(actions),
    );

    // The mono identity line: what it is called on disk, and what it needs.
    let mut identity = div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::SM))
        .pl(px(column::LEAD + space::SM))
        .font_family(font_mono)
        .text_size(px(text::LABEL))
        .text_color(muted)
        .child(script.id.clone())
        .child("·")
        .child(if script.supports_dry_run {
            "-DryRun supported"
        } else {
            "no dry run"
        });

    if let Some(author) = &script.author {
        identity = identity.child("·").child(format!("by {author}"));
    }
    body = body.child(identity);

    let summary = script.summary_text();
    if !summary.is_empty() {
        body = body.child(
            div()
                .max_w(px(760.0))
                .pl(px(column::LEAD + space::SM))
                .font_family(theme.font_ui.clone())
                .text_size(px(text::SMALL))
                .text_color(muted)
                .child(summary),
        );
    }

    if let Some(reason) = script.blocked_reason() {
        body = body.child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(space::XS))
                .pl(px(column::LEAD + space::SM))
                .text_size(px(text::SMALL))
                .child(icon(Icon::Alert, 12.0, theme.danger))
                .child(reason.to_string()),
        );
    }

    body.into_any_element()
}

// ---------------------------------------------------------------------------
// Preset rows
// ---------------------------------------------------------------------------

/// A preset: a name, how many packages it names, and the action.
pub fn preset_row(theme: &Theme, preset: &Preset, actions: AnyElement) -> AnyElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .w_full()
        .gap(px(space::MD))
        .px(px(space::MD))
        .py(px(space::SM + 2.0))
        .border_b_1()
        .border_color(theme.border.opacity(veil::ROW_RULE))
        .child(icon(Icon::Layers, LEAD_ICON, theme.muted_foreground))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(1.0))
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .truncate()
                        .font_family(theme.font_ui.clone())
                        .text_size(px(text::SUBTITLE))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.foreground)
                        .child(preset.name.clone()),
                )
                .child(
                    div()
                        .font_family(theme.font_mono.clone())
                        .text_size(px(text::LABEL))
                        .text_color(theme.muted_foreground)
                        .child(format!("{} packages", preset.len())),
                ),
        )
        .child(actions)
        .into_any_element()
}

// ---------------------------------------------------------------------------
// Inspector rows
// ---------------------------------------------------------------------------

/// One label/value line in an inspector. The label column is fixed so the values
/// line up down the pane.
pub fn description_row(
    theme: &Theme,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    mono: bool,
) -> AnyElement {
    let value = value.into();
    let empty = value.is_empty();
    div()
        .flex()
        .flex_row()
        .items_start()
        .gap(px(space::SM))
        .py(px(3.0))
        .child(
            div()
                .w(px(84.0))
                .flex_shrink_0()
                .text_size(px(text::LABEL))
                .font_family(theme.font_ui.clone())
                .text_color(theme.muted_foreground)
                .child(label.into()),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(text::SMALL))
                .font_family(if mono {
                    theme.font_mono.clone()
                } else {
                    theme.font_ui.clone()
                })
                .text_color(if empty {
                    theme.muted_foreground
                } else {
                    theme.foreground
                })
                .child(if empty {
                    "—".to_string()
                } else {
                    value.to_string()
                }),
        )
        .into_any_element()
}

/// A selected filter row, in a navigator pane: a full-width button with a chevron
/// when it is the one in force.
pub fn nav_row<F>(
    theme: &Theme,
    id: impl Into<gpui::ElementId>,
    mark: Option<Icon>,
    label: impl Into<SharedString>,
    count: Option<usize>,
    active: bool,
    on_click: F,
) -> AnyElement
where
    F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
{
    let label = label.into();
    let fill = if active {
        theme.sidebar_accent
    } else {
        rgba(0x00000000)
    };
    let hover = theme.sidebar_accent.opacity(0.6);
    let text_color = if active {
        theme.foreground
    } else {
        theme.muted_foreground
    };
    let icon_color = text_color;
    let font = theme.font_ui.clone();

    let mut row = div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .w_full()
        .gap(px(space::SM))
        .h(px(metrics::CONTROL))
        .flex_shrink_0()
        .mx(px(space::SM))
        .px(px(space::SM))
        .rounded(px(radius::CONTROL))
        .bg(fill)
        .text_color(text_color)
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(on_click);

    if let Some(mark) = mark {
        row = row.child(icon(mark, LEAD_ICON, icon_color));
    }
    row = row.child(
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .font_family(font)
            .text_size(px(text::SMALL))
            .child(label),
    );
    if let Some(count) = count {
        row = row.child(
            div()
                .font_family(theme.font_mono.clone())
                .text_size(px(text::LABEL))
                .child(count.to_string()),
        );
    }
    row.into_any_element()
}

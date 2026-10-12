//! The inspector — the right column.
//!
//! Off by default, because it is a second opinion rather than the main event. It
//! answers the question a row cannot fit: *what is this thing, and what happens if
//! it goes?*
//!
//! On the Packages surface it describes the package under the keyboard cursor. On
//! the other two it explains the vocabulary of the surface you are standing on — the
//! risk labels, the preset format — because in a tool whose whole job is "don't let
//! me break my machine", a legend is a feature rather than filler.

use crate::components::button::button;
use crate::components::panel::panel_header;
use crate::components::rows::{
    caveat_chip, caveat_detail, description_row, package_caveat, risk_chip,
};
use crate::components::{Tone, chip, eyebrow, pane_hint};
use crate::icons::Icon;
use crate::snapshot::{Screen, Snapshot};
use crate::theme::{Theme, metrics, space, text, veil};
use crate::views::DebrisApp;
use crate::views::shared::{code_block, scroll_body};
use debris_core::{Category, Risk};
use gpui::{AnyElement, ClickEvent, Context, IntoElement, div, prelude::*, px};

pub fn inspector<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    cx: &mut Context<T>,
) -> AnyElement {
    let subtitle = match snapshot.screen {
        Screen::Packages => "The package under the cursor",
        Screen::Scripts => "The library and its vocabulary",
        Screen::Presets => "How a preset is applied",
    };

    let header = panel_header(
        theme,
        "Inspector",
        subtitle,
        Some(
            button(theme, "hide-inspector")
                .ghost()
                .small()
                .icon(Icon::PanelRight)
                .icon_only()
                .tooltip("Hide the inspector")
                .build(
                    cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                        this.toggle_inspector(cx)
                    }),
                ),
        ),
    );

    let body: AnyElement = match snapshot.screen {
        Screen::Packages => package_detail::<T>(theme, snapshot, cx),
        Screen::Scripts => library_detail(theme),
        Screen::Presets => preset_detail(theme, snapshot),
    };

    div()
        .flex()
        .flex_col()
        .w(px(metrics::INSPECTOR))
        .h_full()
        .flex_shrink_0()
        .min_h_0()
        .bg(theme.background)
        .border_l_1()
        .border_color(theme.border)
        .child(header)
        .child(
            scroll_body("inspector-scroll")
                .p(px(space::MD))
                .gap(px(space::LG))
                .child(body),
        )
        .into_any_element()
}

/// A labelled group inside the inspector: an eyebrow, then its block.
fn group(theme: &Theme, label: &str, content: Vec<AnyElement>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(space::XS))
        .child(eyebrow(theme, label.to_string()))
        .children(content)
        .into_any_element()
}

/// A paragraph of explanation, in the inspector's own voice.
fn prose(theme: &Theme, content: &str) -> AnyElement {
    div()
        .font_family(theme.font_ui.clone())
        .text_size(px(text::SMALL))
        .text_color(theme.foreground)
        .child(content.to_string())
        .into_any_element()
}

/// A quieter paragraph: the sentence under a fact that qualifies it.
fn sub_prose(theme: &Theme, content: &str) -> AnyElement {
    div()
        .font_family(theme.font_ui.clone())
        .text_size(px(text::LABEL))
        .text_color(theme.muted_foreground)
        .child(content.to_string())
        .into_any_element()
}

fn package_detail<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    cx: &mut Context<T>,
) -> AnyElement {
    let Some(package) = snapshot.focused() else {
        return pane_hint(theme, "Click a package to inspect it.");
    };

    let staged = snapshot.staged_names.contains(&package.name);
    let name = package.name.clone();

    let facts = group(
        theme,
        "PACKAGE",
        vec![
            description_row(theme, "Name", package.name.clone(), true),
            description_row(
                theme,
                "Publisher",
                package.publisher_short().to_string(),
                false,
            ),
            description_row(theme, "Version", package.version.clone(), true),
            description_row(
                theme,
                "Staged",
                if staged { "Yes, in the queue" } else { "No" },
                false,
            ),
        ],
    );

    let identity = group(
        theme,
        "IDENTITY",
        vec![
            code_block(theme, package.full_name.clone()),
            sub_prose(theme, "This is the string an uninstall is keyed on."),
        ],
    );

    let mut sections = vec![facts, identity];

    if let Some(caveat) = package_caveat(package, staged) {
        sections.push(group(
            theme,
            "NOTE",
            vec![
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(space::SM))
                    .child(caveat_chip(theme, caveat))
                    .into_any_element(),
                prose(theme, caveat_detail(caveat)),
            ],
        ));
    }

    let action = button(theme, "inspector-act")
        .label(if staged {
            "Return to inventory"
        } else {
            "Add to removal queue"
        })
        .icon(if staged { Icon::ArrowLeft } else { Icon::ArrowRight })
        .outline()
        .build(
            cx.listener(move |this: &mut T, _event: &ClickEvent, _window, cx| {
                let name = name.clone();
                if this.snapshot().staged_names.contains(&name) {
                    this.unstage(&name, cx);
                } else {
                    this.stage(&name, cx);
                }
            }),
        );

    sections.push(
        div()
            .flex()
            .flex_col()
            .gap(px(space::SM))
            .child(action)
            .child(shortcuts(theme))
            .into_any_element(),
    );

    div()
        .flex()
        .flex_col()
        .gap(px(space::LG))
        .children(sections)
        .into_any_element()
}

/// The legend. Every colour the app can use on a row, with the sentence that says
/// what it means — so the chips never have to be learned.
fn library_detail(theme: &Theme) -> AnyElement {
    let mut legend: Vec<AnyElement> =
        vec![eyebrow(theme, "RISK LABELS".to_string()).into_any_element()];

    for risk in [Risk::ReadOnly, Risk::Reversible, Risk::Destructive] {
        legend.push(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .py(px(space::XS))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(px(space::SM))
                        .child(risk_chip(theme, risk)),
                )
                .child(sub_prose(theme, risk.detail()))
                .into_any_element(),
        );
    }

    let risk_section = div()
        .flex()
        .flex_col()
        .gap(px(space::XS))
        .children(legend)
        .into_any_element();

    let sources = group(
        theme,
        "WHERE THEY COME FROM",
        vec![
            prose(
                theme,
                "Every script is a *.ps1 under scripts/, described by the ### header at the top of its own file. A script without a valid header is refused, not hidden.",
            ),
            code_block(theme, "scripts/<category>/<name>.ps1"),
        ],
    );

    let categories = group(
        theme,
        "CATEGORIES",
        Category::ALL
            .iter()
            .map(|category| {
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(space::SM))
                    .child(chip(theme, category.label(), Tone::Quiet))
                    .child(
                        div()
                            .font_family(theme.font_mono.clone())
                            .text_size(px(text::LABEL))
                            .text_color(theme.muted_foreground)
                            .child(category.directory().to_string()),
                    )
                    .into_any_element()
            })
            .collect(),
    );

    div()
        .flex()
        .flex_col()
        .gap(px(space::LG))
        .child(risk_section)
        .child(sources)
        .child(categories)
        .into_any_element()
}

fn preset_detail(theme: &Theme, snapshot: &Snapshot) -> AnyElement {
    let total: usize = snapshot.presets.iter().map(|preset| preset.len()).sum();

    let about = group(
        theme,
        "WHAT A PRESET DOES",
        vec![prose(
            theme,
            "Loading a preset replaces the removal queue. It never removes anything. The confirming button stays on the Packages surface.",
        )],
    );

    let counted = group(
        theme,
        "ON THIS MACHINE",
        vec![
            description_row(theme, "Presets", snapshot.presets.len().to_string(), true),
            description_row(theme, "Names", total.to_string(), true),
            description_row(theme, "Queue", snapshot.staged.len().to_string(), true),
        ],
    );

    let format = group(
        theme,
        "FILE FORMAT",
        vec![
            prose(
                theme,
                "One package name per line. Comments start with #. Drop a file into assets/data/presets and it appears here.",
            ),
            code_block(theme, "Microsoft.BingNews\nMicrosoft.BingWeather"),
        ],
    );

    div()
        .flex()
        .flex_col()
        .gap(px(space::LG))
        .child(about)
        .child(counted)
        .child(format)
        .into_any_element()
}

/// The strokes that actually exist in the app, listed only where they apply. A
/// shortcut list is a promise; this one only makes promises the table keeps.
fn shortcuts(theme: &Theme) -> AnyElement {
    let row = |label: &str, stroke: &str| {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .py(px(2.0))
            .child(
                div()
                    .text_size(px(text::LABEL))
                    .text_color(theme.muted_foreground)
                    .child(label.to_string()),
            )
            .child(crate::components::kbd(theme, stroke.to_string()))
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .gap(px(space::XS))
        .border_t_1()
        .border_color(theme.border.opacity(veil::HAIRLINE))
        .pt(px(space::SM))
        .child(eyebrow(theme, "IN THE TABLE".to_string()))
        .child(row("Move", "↑ ↓"))
        .child(row("Stage / return", "Enter"))
        .child(row("Clear the search", "Esc"))
        .into_any_element()
}

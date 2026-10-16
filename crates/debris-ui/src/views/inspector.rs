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
        Screen::Fixers => "The fixer catalogue and its states",
        Screen::Scripts => "The library and its vocabulary",
        Screen::Extensions => "How a catalogue is read",
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
        Screen::Fixers => fixer_detail(theme),
        Screen::Scripts => library_detail(theme),
        Screen::Extensions => extension_detail(theme),
        Screen::Presets => preset_detail(theme),
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
                "Category",
                snapshot
                    .bloat
                    .category_of(&package.name)
                    .unwrap_or("Not catalogued")
                    .to_string(),
                false,
            ),
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

    // What the catalogue says this package is. Advisory text, and the only place the
    // app can answer "do I want this?" rather than "what is this?".
    if let Some(description) = snapshot.bloat.describe(&package.name) {
        sections.push(group(
            theme,
            "WHAT IT IS",
            vec![prose(theme, description)],
        ));
    }

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

/// The fixer legend: the two ways back, and why there is one.
///
/// Deliberately not a legend for the state chips. `already set` and `not set` are
/// plain English and each row carries its own, so a table repeating them would be a
/// second copy of the row to keep in sync, not an explanation.
fn fixer_detail(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(space::LG))
        .child(group(
            theme,
            "HOW A RUN WORKS",
            vec![
                prose(
                    theme,
                    "Ticking a box writes nothing. Run writes every ticked rule in one pass, and reports each one.",
                ),
                prose(
                    theme,
                    "Restore puts a ticked rule back to the value Windows ships with, or removes it where Windows ships nothing.",
                ),
                sub_prose(
                    theme,
                    "That Windows default is what makes the way back possible. A rule without one cannot be restored.",
                ),
            ],
        ))
        .into_any_element()
}

/// The extension legend: what a catalogue is, and the two steps between listing and
/// running.
fn extension_detail(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(space::LG))
        .child(group(
            theme,
            "CATALOGUES",
            vec![
                prose(
                    theme,
                    "A catalogue is a JSON file that lists scripts and the address of each one.",
                ),
                prose(
                    theme,
                    "No script runs until you install it and confirm.",
                ),
            ],
        ))
        .child(group(
            theme,
            "ENTRY FORMAT",
            vec![
                code_block(theme, "{ name, description, url, version, requiresAdmin, sha256 }"),
                sub_prose(
                    theme,
                    "url has to point at a .ps1. When a catalogue publishes a sha256, a download that does not match it is discarded.",
                ),
            ],
        ))
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

fn preset_detail(theme: &Theme) -> AnyElement {

    let about = group(
        theme,
        "WHAT A PRESET DOES",
        vec![prose(
            theme,
            "Loads a list of recommended package uninstalls.",
        )],
    );


    div()
        .flex()
        .flex_col()
        .gap(px(space::LG))
        .child(about)
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

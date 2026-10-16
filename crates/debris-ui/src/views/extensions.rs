//! The Extensions surface: third-party scripts, listed from a JSON catalogue.
//!
//! # The model
//!
//! A catalogue is a URL. The user adds one, the app fetches it, and every entry it
//! lists becomes a row. Nothing is fetched until asked, and nothing is *run* until
//! installed and confirmed, which is two deliberate steps: the catalogue is somebody
//! else's content, so the app never treats "listed" as "trusted".
//!
//! # Why the sources are editable here and not in a settings pane
//!
//! The control that changes a list belongs next to the list. A settings page a
//! navigation away from the thing it configures is how a feature becomes something
//! users do not find.

use crate::components::button::{Variant, button};
use crate::components::panel::{panel_body, panel_header};
use crate::components::rows;
use crate::components::text_field::{Text, text_field};
use crate::components::{Tone, chip, empty_state, section_eyebrow};
use crate::icons::Icon;
use crate::snapshot::{CatalogEntry, Screen, Snapshot};
use crate::theme::{Theme, metrics, space, text, veil};
use crate::views::{DebrisApp, shared};
use debris_core::is_fetchable;
use gpui::{AnyElement, ClickEvent, Context, FocusHandle, IntoElement, div, prelude::*, px};

/// The catalogue every new profile can start from. Empty on purpose: the shipped
/// build lists no third-party scripts, so the surface begins by asking for a link
/// rather than offering something nobody chose.
const PLACEHOLDER: &str = "https://example.com/catalog.json";

pub fn screen<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    source_focus: &FocusHandle,
    cx: &mut Context<T>,
) -> AnyElement {
    let visible: Vec<&CatalogEntry> = snapshot
        .extensions
        .iter()
        .filter(|entry| matches(entry, &snapshot.query))
        .collect();

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .bg(theme.background)
        .child(panel_header(
            theme,
            Screen::Extensions.title(),
            subtitle(snapshot, visible.len()),
            None,
        ))
        .child(toolbar::<T>(theme, snapshot, source_focus, cx))
        .child(body::<T>(theme, snapshot, &visible, cx))
        .into_any_element()
}

fn matches(entry: &CatalogEntry, query: &str) -> bool {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return true;
    }
    entry.extension.name.to_ascii_lowercase().contains(&query)
        || entry
            .extension
            .description
            .to_ascii_lowercase()
            .contains(&query)
        || entry.source.to_ascii_lowercase().contains(&query)
}

fn subtitle(snapshot: &Snapshot, shown: usize) -> String {
    let total = snapshot.extensions.len();
    let sources = snapshot.extension_sources.len();
    let installed = snapshot.installed_extensions.len();
    let mut parts = vec![shared::count(total, "extension", "extensions")];
    parts.push(shared::count(sources, "catalogue", "catalogues"));
    if installed > 0 {
        parts.push(format!("{installed} installed"));
    }
    if shown != total {
        parts.push(format!("{shown} shown"));
    }
    parts.join(" · ")
}

/// The catalogue field and the two actions that operate on the whole list.
fn toolbar<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    source_focus: &FocusHandle,
    cx: &mut Context<T>,
) -> AnyElement {
    let busy = snapshot.is_busy();
    let typed = !snapshot.source_input.trim().is_empty();
    // A link is only worth offering to add when it looks like one, so the button
    // says no instead of failing later.
    let addable = typed && is_fetchable(&snapshot.source_input);

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
        .child(
            div()
                .flex_1()
                .min_w_0()
                .max_w(px(520.0))
                .child(text_field(
                    theme,
                    "extension-source",
                    Text {
                        value: &snapshot.source_input,
                        selected: snapshot.source_selected,
                    },
                    PLACEHOLDER,
                    source_focus,
                    shared::source_key_handler(cx),
                    cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                        this.set_source_input(String::new(), cx)
                    }),
                )),
        )
        .child(
            button(theme, "extension-add")
                .label("Add catalogue")
                .icon(Icon::Link)
                .outline()
                .small()
                .disabled(busy || !addable)
                .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.add_source(cx)
                })),
        )
        .child(div().flex_1())
        .when(!snapshot.extension_sources.is_empty(), |bar| {
            bar.child(
                button(theme, "extensions-refresh")
                    .label("Refresh all")
                    .icon(Icon::Refresh)
                    .outline()
                    .small()
                    .disabled(busy)
                    .tooltip("Fetch every catalogue again")
                    .build(cx.listener(|this: &mut T, _event: &ClickEvent, _window, cx| {
                        this.refresh_catalogues(cx)
                    })),
            )
        })
        .child(chip(
            theme,
            shared::count(snapshot.extension_sources.len(), "catalogue", "catalogues"),
            Tone::Quiet,
        ))
        .into_any_element()
}

fn body<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    visible: &[&CatalogEntry],
    cx: &mut Context<T>,
) -> AnyElement {
    if snapshot.extension_sources.is_empty() {
        return empty_state(
            theme,
            Icon::Puzzle,
            "No catalogues yet",
            "Add a link to a JSON catalogue.",
            None,
        );
    }

    let mut list = shared::scroll_body("extensions-scroll");

    // The sources themselves, with a way to forget each one. Above the entries rather
    // than mixed in, because these are the app's configuration and the entries below
    // are somebody else's content.
    list = list.child(section_eyebrow(
        theme,
        "Catalogues",
        Some(snapshot.extension_sources.len()),
    ));
    for source in snapshot.extension_sources.iter() {
        let url = source.clone();
        let loaded = snapshot
            .extensions
            .iter()
            .filter(|entry| &entry.source == source)
            .count();
        list = list.child(rows::source_row(
            theme,
            source,
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(space::SM))
                .child(
                    div()
                        .flex_shrink_0()
                        .font_family(theme.font_mono.clone())
                        .text_size(px(text::LABEL))
                        .text_color(theme.muted_foreground)
                        .child(format!("{loaded} listed")),
                )
                .child(
                    button(theme, format!("source-remove-{url}"))
                        .label("Remove")
                        .danger_ghost()
                        .small()
                        .disabled(snapshot.is_busy())
                        .build(cx.listener(move |this: &mut T, _e: &ClickEvent, _w, cx| {
                            this.remove_source(&url, cx)
                        })),
                )
                .into_any_element(),
        ));
    }

    if visible.is_empty() {
        list = list.child(
            div()
                .p(px(space::LG))
                .font_family(theme.font_ui.clone())
                .text_size(px(text::SMALL))
                .text_color(theme.muted_foreground)
                .child(if snapshot.extensions.is_empty() {
                    "Nothing fetched yet. Press Refresh all."
                } else {
                    "No entry matches that filter."
                }),
        );
        return panel_body().child(list).into_any_element();
    }

    // Grouped by source, so a row always has a visible author.
    let mut sources: Vec<&str> = Vec::new();
    for entry in visible {
        if !sources.contains(&entry.source.as_str()) {
            sources.push(&entry.source);
        }
    }
    for source in sources {
        let group: Vec<&&CatalogEntry> = visible
            .iter()
            .filter(|entry| entry.source == source)
            .collect();
        list = list.child(section_eyebrow(theme, source, Some(group.len())));
        for entry in group {
            let installed = snapshot.installed_extensions.get(&entry.extension.url);
            list = list.child(rows::extension_row(
                theme,
                &entry.extension,
                installed.map(String::as_str),
                action::<T>(theme, snapshot, entry, cx),
            ));
        }
    }

    panel_body().child(list).into_any_element()
}

/// Install, or run what was installed. Never both for one entry.
fn action<T: DebrisApp>(
    theme: &Theme,
    snapshot: &Snapshot,
    entry: &CatalogEntry,
    cx: &mut Context<T>,
) -> AnyElement {
    let url = entry.extension.url.clone();
    let busy = snapshot.is_busy();
    let installed = snapshot.installed_extensions.contains_key(&url);

    if !installed {
        let fetchable = entry.extension.is_fetchable();
        return button(theme, format!("extension-install-{url}"))
            .label("Install")
            .icon(Icon::Download)
            .outline()
            .small()
            .disabled(busy || !fetchable)
            .tooltip(if fetchable {
                "Download this script into the app's own folder"
            } else {
                "This entry is not an http(s) address"
            })
            .build(cx.listener(move |this: &mut T, _event: &ClickEvent, _window, cx| {
                this.install_extension(&url, cx)
            }))
            .into_any_element();
    }

    // Installed, so the two things left are running it and getting rid of it. The
    // second is a ghost: removing a download is not a destructive act.
    let run_url = url.clone();
    let remove_url = url.clone();
    div()
        .flex()
        .flex_row()
        .items_center()
        .gap(px(space::XS))
        .child(
            button(theme, format!("extension-run-{url}"))
                .label("Run")
                .icon(Icon::ArrowRight)
                .variant(if entry.extension.requires_admin {
                    Variant::Danger
                } else {
                    Variant::Primary
                })
                .small()
                .disabled(busy)
                .build(cx.listener(move |this: &mut T, _event: &ClickEvent, window, cx| {
                    this.request_extension(&run_url, window, cx)
                })),
        )
        .child(
            button(theme, format!("extension-remove-{url}"))
                .label("Remove")
                .ghost()
                .small()
                .disabled(busy)
                .build(cx.listener(move |this: &mut T, _event: &ClickEvent, _window, cx| {
                    this.uninstall_extension(&remove_url, cx)
                })),
        )
        .into_any_element()
}

//! Rows. Each one declares its own height, its own hairline, and its own hover — a
//! list in this interface is a stack of rules, not a card with padding.

use crate::components::tag;
use crate::icons::{Icon, icon};
use crate::theme::{Theme, layout, text};
use debris_core::{AppxPackage, Caveat, Preset, Risk, ScriptMeta};
use gpui::{
    AnyElement, App, ClickEvent, FontWeight, IntoElement, Window, div, prelude::*, px, rgba,
};

/// A package in either column.
///
/// The inventory and the removal column use the same component, so a staged row is
/// visibly the *same* row somewhere else rather than a different-looking thing. The
/// only differences are the accent and the marker.
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
    let background = if staged {
        theme.accent_wash()
    } else if selected {
        theme.surface_selected
    } else {
        rgba(0x00000000)
    };
    let hover = if staged {
        theme.accent_wash()
    } else {
        theme.surface_hover
    };
    let marker = if staged {
        theme.accent
    } else if selected {
        theme.ink
    } else {
        rgba(0x00000000)
    };
    let name_color = if staged { theme.accent } else { theme.ink };
    let meta = package.meta_line();
    let caveat = caveat_for(package, staged);

    div()
        .id(id)
        .flex()
        .items_center()
        .gap_3()
        .h(px(layout::ROW))
        .pl_3()
        .pr_3()
        .bg(background)
        .border_b_1()
        .border_color(theme.rule)
        .cursor_pointer()
        .hover(move |style| style.bg(hover))
        .on_click(on_click)
        .child(
            div()
                .w(px(2.0))
                .h(px(14.0))
                .flex_shrink_0()
                .bg(marker),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .font_family(theme.font_mono.clone())
                .text_size(px(text::SMALL))
                .text_color(name_color)
                .truncate()
                .child(package.name.clone()),
        )
        .child(
            div()
                .max_w(px(260.0))
                .font_family(theme.font_mono.clone())
                .text_size(px(text::LABEL))
                .text_color(theme.ink_faint)
                .truncate()
                .child(meta),
        )
        .child(if let Some(caveat) = caveat {
            caveat_tag(theme, caveat).into_any_element()
        } else {
            div().into_any_element()
        })
        .into_any_element()
}

/// A caveat as a tag. `SYSTEM` is a statement of fact; `IRREPLACEABLE` is the one
/// that occasionally stops someone.
pub fn caveat_tag(theme: &Theme, caveat: Caveat) -> impl IntoElement {
    let (foreground, background) = match caveat {
        Caveat::SystemCritical => (theme.warning, theme.warning_soft),
        Caveat::Store => (theme.accent, theme.accent_soft),
    };
    div()
        .flex()
        .items_center()
        .gap_1()
        .text_color(foreground)
        .child(icon(Icon::Warning, 11.0))
        .child(tag(theme, caveat.label(), foreground, background))
}

fn caveat_for(package: &AppxPackage, staged: bool) -> Option<Caveat> {
    // The caveat is only worth showing where it changes a decision: when the row is
    // staged, or when it is a system component the user has chosen to reveal.
    if package.name.eq_ignore_ascii_case("Microsoft.WindowsStore") {
        return Some(Caveat::Store);
    }
    if package.is_system {
        return Some(Caveat::SystemCritical);
    }
    let _ = staged;
    None
}

/// A script in the library. Taller than a package row, because it carries prose.
pub fn script_row(theme: &Theme, script: &ScriptMeta, actions: AnyElement) -> AnyElement {
    let risk = script.risk;
    let font_ui = theme.font_ui.clone();
    let font_mono = theme.font_mono.clone();

    let mut body = div()
        .flex()
        .flex_col()
        .gap_2()
        .px_4()
        .py_3()
        .border_b_1()
        .border_color(theme.rule);

    body = body.child(
        div()
            .flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .font_family(font_ui.clone())
                    .text_size(px(text::SUBTITLE))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.ink)
                    .child(script.title.clone()),
            )
            .child(tag(
                theme,
                script.category.label(),
                theme.ink_muted,
                theme.surface_hover,
            ))
            .child(risk_tag(theme, risk)),
    );

    // The mono identity line: what it is called on disk, and what it needs.
    let mut identity = div()
        .flex()
        .items_center()
        .gap_3()
        .font_family(font_mono)
        .text_size(px(text::LABEL))
        .text_color(theme.ink_faint)
        .child(script.id.clone());

    identity = identity.child(if script.requires_admin {
        "needs administrator".to_string()
    } else {
        "no elevation".to_string()
    });

    identity = identity.child(if script.supports_dry_run {
        "supports -DryRun".to_string()
    } else {
        "no dry run".to_string()
    });

    if let Some(author) = &script.author {
        identity = identity.child(format!("by {author}"));
    }

    body = body.child(identity);

    let summary = script.summary_text();
    if !summary.is_empty() {
        body = body.child(
            div()
                .max_w(px(680.0))
                .font_family(font_ui)
                .text_size(px(text::SMALL))
                .text_color(theme.ink_muted)
                .child(summary),
        );
    }

    if let Some(reason) = script.blocked_reason() {
        body = body.child(
            div()
                .text_size(px(text::SMALL))
                .text_color(theme.accent)
                .child(reason.to_string()),
        );
    }

    body.child(div().flex().items_center().gap_2().child(actions))
        .into_any_element()
}

/// The risk tag. The only place colour appears besides removal.
pub fn risk_tag(theme: &Theme, risk: Risk) -> AnyElement {
    let (foreground, background) = match risk {
        Risk::ReadOnly => (theme.ink_muted, theme.surface_hover),
        Risk::Reversible => (theme.ink, theme.surface_hover),
        Risk::Destructive => (theme.accent, theme.accent_soft),
    };
    tag(theme, risk.label(), foreground, background).into_any_element()
}

/// A preset: a name, how many packages it names, and the action.
pub fn preset_row(theme: &Theme, preset: &Preset, actions: AnyElement) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_4()
        .px_4()
        .py_3()
        .border_b_1()
        .border_color(theme.rule)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .font_family(theme.font_ui.clone())
                        .text_size(px(text::SUBTITLE))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.ink)
                        .child(preset.name.clone()),
                )
                .child(
                    div()
                        .font_family(theme.font_mono.clone())
                        .text_size(px(text::LABEL))
                        .text_color(theme.ink_faint)
                        .child(format!("{} package(s)", preset.len())),
                ),
        )
        .child(actions)
        .into_any_element()
}

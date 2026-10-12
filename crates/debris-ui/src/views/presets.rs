//! The Presets surface: named package sets.

use crate::components::button::{ButtonVariant, button};
use crate::components::rows;
use crate::components::{empty_state, rule};
use crate::snapshot::{Screen, Snapshot};
use crate::theme::Theme;
use crate::views::{DebrisApp, section_title};
use gpui::{AnyElement, ClickEvent, Context, IntoElement, div, prelude::*, px};

pub fn screen<T: DebrisApp>(theme: &Theme, snapshot: &Snapshot, cx: &mut Context<T>) -> AnyElement {
    // An id is required before `overflow_scroll` (it is a `StatefulInteractiveElement`
    // method).
    let mut list = div()
        .id("presets-scroll")
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .overflow_scroll();

    if snapshot.presets.is_empty() {
        list = list.child(empty_state(
            theme,
            "No presets",
            "A preset is a text file of package names, one per line. Drop one into assets/data/presets and it appears here.",
        ));
    } else {
        for preset in snapshot.presets.iter() {
            let name = preset.name.clone();
            let actions = div()
                .flex()
                .items_center()
                .gap_2()
                .child(button(
                    theme,
                    format!("load-preset-{}", preset.name),
                    "Load into staging",
                    ButtonVariant::Default,
                    true,
                    cx.listener(move |this: &mut T, _event: &ClickEvent, _window, cx| {
                        this.load_preset(&name, cx)
                    }),
                ))
                .into_any_element();
            list = list.child(rows::preset_row(theme, preset, actions));
        }
    }

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .child(section_title(theme, Screen::Presets))
        .child(rule(theme))
        .child(
            div()
                .font_family(theme.font_ui.clone())
                .text_size(px(crate::theme::text::LABEL))
                .text_color(theme.ink_muted)
                .px_6()
                .py_3()
                .flex_shrink_0()
                .child(
                    "Loading a preset replaces what is staged. Nothing is removed until you confirm on the Packages surface.",
                ),
        )
        .child(rule(theme))
        .child(list)
        .into_any_element()
}

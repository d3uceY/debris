//! Pieces the three surfaces share: the title block, and the virtualized list.

use crate::components::rows;
use crate::snapshot::Screen;
use crate::theme::{Theme, text};
use crate::views::DebrisApp;
use debris_core::AppxPackage;
use gpui::{
    AnyElement, App, ClickEvent, FontWeight, IntoElement, WeakEntity, Window, div, prelude::*,
    px, uniform_list,
};
use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

/// The surface title, flush left over its blurb, with room above it. Every surface
/// opens the same way, so the eye always lands in the same place.
pub fn section_title(theme: &Theme, screen: Screen) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .px_6()
        .pt_6()
        .pb_4()
        .child(
            div()
                .font_family(theme.font_ui.clone())
                .text_size(px(text::DISPLAY))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme.ink)
                .child(screen.label()),
        )
        .child(
            div()
                .max_w(px(620.0))
                .font_family(theme.font_ui.clone())
                .text_size(px(text::SMALL))
                .text_color(theme.ink_muted)
                .child(screen.blurb()),
        )
        .into_any_element()
}

/// The virtualized package list, used by both columns.
///
/// `uniform_list` only renders the rows the viewport can see, which is what keeps the
/// frame budget flat whether the inventory holds twenty packages or five thousand.
/// It needs a parent with `overflow_hidden` and a bounded height — hence the wrapper.
///
/// The click handler has to be built per row, so it captures a `WeakEntity` of the
/// app rather than a borrow: the closure must be `'static` for the list to hold it.
pub fn package_list<T: DebrisApp>(
    theme: &Theme,
    id: &'static str,
    packages: Arc<Vec<AppxPackage>>,
    staged_names: Arc<HashSet<String>>,
    in_removal_column: bool,
    cursor: Option<usize>,
    root: &WeakEntity<T>,
) -> AnyElement {
    let theme = theme.clone();
    let root = root.clone();
    let item_count = packages.len();

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .overflow_hidden()
        .child(
            uniform_list(
                id,
                item_count,
                move |range: Range<usize>, _window: &mut Window, _cx: &mut App| -> Vec<AnyElement> {
                    let mut items: Vec<AnyElement> = Vec::with_capacity(range.len());
                    for index in range {
                        let Some(package) = packages.get(index) else {
                            continue;
                        };
                        let name = package.name.clone();
                        let staged = staged_names.contains(&name);
                        let selected = cursor == Some(index);
                        let entity = root.clone();
                        let on_click = move |_event: &ClickEvent,
                                             _window: &mut Window,
                                             cx: &mut App| {
                            let name = name.clone();
                            if in_removal_column {
                                entity.update(cx, |this, cx| this.unstage(&name, cx)).ok();
                            } else {
                                entity.update(cx, |this, cx| this.stage(&name, cx)).ok();
                            }
                        };
                        items.push(rows::package_row(
                            &theme,
                            if in_removal_column { ("staged", index) } else { ("inventory", index) },
                            package,
                            staged,
                            selected,
                            on_click,
                        ));
                    }
                    items
                },
            )
            .flex_1(),
        )
        .into_any_element()
}

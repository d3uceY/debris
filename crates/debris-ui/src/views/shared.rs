//! Pieces the surfaces share: the key handlers, the virtualized table, the scroll
//! body, and the small blocks an inspector is built from.

use crate::components::rows;
use crate::theme::{Theme, space, text, veil};
use crate::views::DebrisApp;
use debris_core::AppxPackage;
use gpui::{
    AnyElement, App, ClickEvent, IntoElement, KeyDownEvent, SharedString, WeakEntity, Window, div,
    prelude::*, px, uniform_list,
};
use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

/// The two strokes the inventory listens for, shared by both package views.
pub fn list_key_handler<T: DebrisApp>(
    cx: &mut gpui::Context<T>,
) -> impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static {
    cx.listener(|this: &mut T, event: &KeyDownEvent, _window, cx| {
        match event.keystroke.key.as_str() {
            "up" => this.move_cursor(-1, cx),
            "down" => this.move_cursor(1, cx),
            "enter" | "space" => this.activate_cursor(cx),
            _ => {}
        }
    })
}

/// Typing in the one search field. Kept here rather than in a field component
/// because the field does not own the string — the app does.
pub fn search_key_handler<T: DebrisApp>(
    cx: &mut gpui::Context<T>,
) -> impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static {
    cx.listener(|this: &mut T, event: &KeyDownEvent, _window, cx| {
        let current = this.snapshot().query.clone();
        match event.keystroke.key.as_str() {
            "backspace" => {
                let mut next = current;
                next.pop();
                this.set_query(next, cx);
            }
            "escape" | "delete" => this.set_query(String::new(), cx),
            _ => {
                if let Some(typed) = event.keystroke.key_char.as_deref()
                    && !typed.is_empty()
                    && !typed.chars().any(char::is_control)
                {
                    let mut next = current;
                    next.push_str(typed);
                    this.set_query(next, cx);
                }
            }
        }
    })
}

/// Typing in the catalogue field on the Extensions surface.
///
/// A second field, and the only one: the rule that a shell has one search field is
/// about *filters*, and this is not a filter. It is the value a button acts on, which
/// is why it sits beside that button instead of in the title bar.
pub fn source_key_handler<T: DebrisApp>(
    cx: &mut gpui::Context<T>,
) -> impl Fn(&KeyDownEvent, &mut Window, &mut App) + 'static {
    cx.listener(|this: &mut T, event: &KeyDownEvent, _window, cx| {
        let current = this.snapshot().source_input.clone();
        match event.keystroke.key.as_str() {
            "enter" => this.add_source(cx),
            "backspace" => {
                let mut next = current;
                next.pop();
                this.set_source_input(next, cx);
            }
            "escape" | "delete" => this.set_source_input(String::new(), cx),
            _ => {
                if let Some(typed) = event.keystroke.key_char.as_deref()
                    && !typed.is_empty()
                    && !typed.chars().any(char::is_control)
                {
                    let mut next = current;
                    next.push_str(typed);
                    this.set_source_input(next, cx);
                }
            }
        }
    })
}

/// A scroll region. `overflow_scroll` lives on `StatefulInteractiveElement`, so the
/// container needs an element id before it can scroll — and taking an id is what
/// turns a `Div` into a `Stateful<Div>`.
pub fn scroll_body(id: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .w_full()
        .overflow_scroll()
}

/// The virtualized package table, used by the inventory and the removal queue.
///
/// `uniform_list` only renders the rows the viewport can see, which is what keeps
/// the frame budget flat whether the inventory holds twenty packages or five
/// thousand. It needs a parent with `overflow_hidden` and a bounded height.
///
/// The click handler has to be built per row, so it captures a `WeakEntity` of the
/// app rather than a borrow: the closure must be `'static` for the list to hold it.
pub fn package_table<T: DebrisApp>(
    theme: &Theme,
    id: &'static str,
    packages: Arc<Vec<AppxPackage>>,
    staged_names: Arc<HashSet<String>>,
    in_queue: bool,
    cursor: Option<usize>,
    root: &WeakEntity<T>,
) -> AnyElement {
    let theme = theme.clone();
    let root = root.clone();
    let item_count = packages.len();

    let list = uniform_list(
        id,
        item_count,
        move |range: Range<usize>,
              _window: &mut Window,
              _cx: &mut App|
              -> Vec<AnyElement> {
            let mut items: Vec<AnyElement> = Vec::with_capacity(range.len());
            for index in range {
                let Some(package) = packages.get(index) else {
                    continue;
                };
                let name = package.name.clone();
                let staged = staged_names.contains(&name);
                let selected = cursor == Some(index);
                let entity = root.clone();
                let on_click =
                    move |_event: &ClickEvent, _window: &mut Window, cx: &mut App| {
                        let name = name.clone();
                        if in_queue {
                            entity.update(cx, |this, cx| this.unstage(&name, cx)).ok();
                        } else {
                            entity.update(cx, |this, cx| this.stage(&name, cx)).ok();
                        }
                    };
                items.push(rows::package_row(
                    &theme,
                    if in_queue {
                        ("staged", index)
                    } else {
                        ("inventory", index)
                    },
                    package,
                    staged,
                    selected,
                    on_click,
                ));
            }
            items
        },
    )
    .flex_1();

    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .w_full()
        .overflow_hidden()
        .child(list)
        .into_any_element()
}

/// `1 package` / `3 packages`.
///
/// An app that writes "1 packages" is an app nobody read twice, and this interface
/// counts things on almost every row. One helper, used everywhere a number meets a
/// noun.
pub fn count(n: usize, singular: &str, plural: &str) -> String {
    if n == 1 {
        format!("{n} {singular}")
    } else {
        format!("{n} {plural}")
    }
}

/// A wrapped monospace block for a long identifier that has no spaces to break on.
/// The identifier this tool displays — an Appx package family name — is 60+
/// characters of unbroken text, so it needs a block rather than a row.
pub fn code_block(theme: &Theme, value: impl Into<SharedString>) -> AnyElement {
    div()
        .w_full()
        .px(px(space::SM))
        .py(px(space::XS + 2.0))
        .rounded(px(crate::theme::radius::CHIP))
        .bg(theme.band(veil::CHIP))
        .font_family(theme.font_mono.clone())
        .text_size(px(text::LABEL))
        .text_color(theme.foreground)
        .child(value.into())
        .into_any_element()
}

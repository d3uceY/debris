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
        let selected = this.snapshot().query_selected;
        // Bound to a local so the closure's borrow of `cx` ends before the setter
        // needs it mutably.
        let change = edit(&current, selected, event, || clipboard_text(&*cx));
        match change {
            Some(Edit::SelectAll) => this.select_query(cx),
            Some(Edit::Value(value)) => this.set_query(value, cx),
            None => {}
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
        if event.keystroke.key == "enter" {
            this.add_source(cx);
            return;
        }
        let current = this.snapshot().source_input.clone();
        let selected = this.snapshot().source_selected;
        let change = edit(&current, selected, event, || clipboard_text(&*cx));
        match change {
            Some(Edit::SelectAll) => this.select_source(cx),
            Some(Edit::Value(value)) => this.set_source_input(value, cx),
            None => {}
        }
    })
}

/// What a keystroke means to a one-line field.
#[derive(Debug, PartialEq)]
enum Edit {
    /// The new value. The selection, if there was one, is gone.
    Value(String),
    /// Ctrl+A: the whole value is now selected.
    SelectAll,
}

/// The rule every one-line field in the app edits by, in one place: `None` for a key
/// the field ignores.
///
/// The field has no cursor to move — the caret always trails — so “selected” can only
/// mean the whole value, and a field is either fully selected or not selected at all.
/// `paste` is a closure rather than a `&str` because reading the clipboard needs the
/// app, which a test cannot build — it is only consulted for Ctrl+V.
fn edit<P>(
    current: &str,
    selected: bool,
    event: &KeyDownEvent,
    paste: P,
) -> Option<Edit>
where
    P: FnOnce() -> Option<String>,
{
    let keystroke = &event.keystroke;
    // A chorded key has no `key_char` — Ctrl+A and Ctrl+V are only ever the key name.
    // AltGr is Ctrl+Alt on Windows and *does* type, hence the `!alt`.
    let chord = keystroke.modifiers.control && !keystroke.modifiers.alt;
    // Everything that replaces the value: an edit ends the selection.
    let replacing = selected.then(String::new).unwrap_or_else(|| current.to_string());

    match keystroke.key.as_str() {
        _ if chord && keystroke.key == "a" => {
            // Nothing to select is not a selection.
            (!current.is_empty()).then_some(Edit::SelectAll)
        }
        _ if chord && keystroke.key == "v" => {
            // Control characters are dropped rather than rejected: a copied paragraph
            // still lands in the field, flattened onto one line.
            let pasted = paste()?;
            let mut value = replacing;
            value.extend(pasted.chars().filter(|c| !c.is_control()));
            Some(Edit::Value(value))
        }
        // Backspace over a selection deletes it; otherwise it deletes one character.
        "backspace" => {
            let mut value = replacing;
            value.pop();
            Some(Edit::Value(value))
        }
        "escape" | "delete" => Some(Edit::Value(String::new())),
        _ => {
            let typed = keystroke.key_char.as_deref().unwrap_or_default();
            if typed.is_empty() || typed.chars().any(char::is_control) {
                return None;
            }
            let mut value = replacing;
            value.push_str(typed);
            Some(Edit::Value(value))
        }
    }
}

/// The clipboard as plain text, or nothing if it holds an image, files, or nothing
/// we can read.
fn clipboard_text(cx: &App) -> Option<String> {
    cx.read_from_clipboard().and_then(|item| item.text())
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

// Declared in `components` so a component row can pluralise too; re-exported here
// because every surface already reaches for it through this module.
pub use crate::components::count;

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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Keystroke, Modifiers};

    fn pressed(key: &str, modifiers: Modifiers, key_char: Option<&str>) -> KeyDownEvent {
        KeyDownEvent {
            keystroke: Keystroke {
                modifiers,
                key: key.to_string(),
                key_char: key_char.map(ToString::to_string),
            },
            is_held: false,
            prefer_character_input: false,
        }
    }

    fn value(next: Option<Edit>) -> Option<String> {
        match next {
            Some(Edit::Value(value)) => Some(value),
            _ => None,
        }
    }

    /// The ways a field can change, and the one keystroke that must not: Ctrl+V
    /// pastes *flattened*, and no other chord reaches the clipboard or the string.
    #[test]
    fn a_field_pastes_and_ignores_every_other_chord() {
        let no_clipboard = || None;
        let pasted = "https://example.com/a\tb\r\nc";

        // Ctrl+V: tabs and breaks come off, the URL survives.
        assert_eq!(
            value(edit("", false, &pressed("v", Modifiers::control(), None), || {
                Some(pasted.into())
            })),
            Some("https://example.com/abc".to_string())
        );
        // An empty clipboard is a no-op, never a cleared field.
        assert_eq!(
            edit("ab", false, &pressed("v", Modifiers::control(), None), no_clipboard),
            None
        );
        // An unbound chord does nothing, and does not read the clipboard.
        assert_eq!(
            edit("ab", false, &pressed("q", Modifiers::control(), None), || panic!("read")),
            None
        );
        // The ordinary paths still work.
        assert_eq!(
            value(edit("a", false, &pressed("b", Modifiers::none(), Some("b")), no_clipboard)),
            Some("ab".to_string())
        );
        assert_eq!(
            value(edit("ab", false, &pressed("backspace", Modifiers::none(), None), no_clipboard)),
            Some("a".to_string())
        );
        assert_eq!(
            value(edit("ab", false, &pressed("escape", Modifiers::none(), None), no_clipboard)),
            Some(String::new())
        );
        // A paste that is not Ctrl+V — AltGr+V on a layout that types with it.
        assert_eq!(
            value(edit("a", false, &pressed("v", Modifiers::alt(), Some("v")), || panic!("read"))),
            Some("av".to_string())
        );
    }

    /// Ctrl+A selects the whole value — the only selection this field can have, since
    /// there is no cursor to move — and the next key replaces it.
    #[test]
    fn a_selection_is_replaced_by_the_next_key() {
        let no_clipboard = || None;
        let select = |current: &str| edit(current, false, &pressed("a", Modifiers::control(), None), no_clipboard);

        assert_eq!(select("ab"), Some(Edit::SelectAll));
        // Nothing typed is nothing to select, so Ctrl+A is not a change.
        assert_eq!(select(""), None);

        let selected = |key: &str, key_char: Option<&str>| {
            value(edit(
                "ab",
                true,
                &pressed(key, Modifiers::none(), key_char),
                no_clipboard,
            ))
        };
        assert_eq!(selected("backspace", None), Some(String::new()));
        assert_eq!(selected("c", Some("c")), Some("c".to_string()));
        // Pasting over a selection replaces it, rather than appending to it.
        assert_eq!(
            value(edit("ab", true, &pressed("v", Modifiers::control(), None), || {
                Some("z".to_string())
            })),
            Some("z".to_string())
        );
    }
}

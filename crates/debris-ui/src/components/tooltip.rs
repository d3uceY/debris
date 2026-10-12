//! Tooltips.
//!
//! GPUI has no tooltip *element*; an element asks the window for a view to show
//! (`StatefulInteractiveElement::tooltip`). So a tooltip here is a tiny `Render`
//! view plus a one-line adapter that hands it to the window.
//!
//! ```ignore
//! div().id("x").tooltip(tip("Rescan the inventory"))
//! ```
//!
//! A tooltip is drawn as a floating surface — page background, hairline, and the one
//! shadow in the interface. It has to lift clear of whatever it covers, and a
//! hairline alone cannot do that.

use crate::theme::{ThemeExt, radius, text};
use gpui::{
    AnyView, App, AppContext, Context, IntoElement, Render, SharedString, Window, div, prelude::*,
    px,
};

pub struct Tip {
    text: SharedString,
}

impl Tip {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl Render for Tip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(px(6.0))
            .px(px(8.0))
            .py(px(4.0))
            .rounded(px(radius::CHIP))
            .border_1()
            .border_color(theme.border)
            .bg(theme.background)
            .shadow_md()
            .font_family(theme.font_ui.clone())
            .text_size(px(text::SMALL))
            .text_color(theme.foreground)
            .child(self.text.clone())
    }
}

/// A tooltip builder for `StatefulInteractiveElement::tooltip`.
pub fn tip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_window, cx| cx.new(|_| Tip::new(text.clone())).into()
}

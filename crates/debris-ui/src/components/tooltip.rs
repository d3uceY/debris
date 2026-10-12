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

use crate::theme::{Theme, ThemeExt, radius, text};
use gpui::{
    AnyView, App, AppContext, Context, IntoElement, Render, SharedString, Window, div, prelude::*,
    px,
};

pub struct Tip {
    text: SharedString,
    /// A keycap shown after the sentence, when the action also has a binding.
    stroke: Option<SharedString>,
}

impl Tip {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self {
            text: text.into(),
            stroke: None,
        }
    }

    pub fn with_stroke(mut self, stroke: impl Into<SharedString>) -> Self {
        self.stroke = Some(stroke.into());
        self
    }
}

impl Render for Tip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let mut row = div()
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
            .child(self.text.clone());

        if let Some(stroke) = self.stroke.clone() {
            row = row.child(keycap(theme, stroke));
        }
        row
    }
}

fn keycap(theme: &Theme, stroke: SharedString) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .h(px(15.0))
        .px(px(4.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(theme.border)
        .font_family(theme.font_mono.clone())
        .text_size(px(text::LABEL))
        .text_color(theme.muted_foreground)
        .child(stroke)
}

/// A tooltip builder for `StatefulInteractiveElement::tooltip`.
pub fn tip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_window, cx| cx.new(|_| Tip::new(text.clone())).into()
}

/// The same, with a keycap.
pub fn tip_with(
    text: impl Into<SharedString>,
    stroke: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    let stroke = stroke.into();
    move |_window, cx| cx.new(|_| Tip::new(text.clone()).with_stroke(stroke.clone())).into()
}

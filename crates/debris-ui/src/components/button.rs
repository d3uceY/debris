//! Buttons.
//!
//! Five variants are the whole vocabulary, and the rule for picking one is
//! positional rather than decorative:
//!
//! * **Primary** — the single action that moves the task forward. One per screen.
//! * **Outline** — an ordinary action with a shape.
//! * **Ghost** — a tertiary action, or anything in the title bar. It has no shape
//!   until you hover it.
//! * **Danger** — the destructive action.
//! * **DangerGhost** — a destructive *escape* (Clear, Discard): red text, no fill.
//!
//! A disabled control stays legible instead of fading to invisibility: it keeps its
//! shape and its label, and loses only its weight.

use crate::icons::{Icon, icon};
use crate::theme::{Theme, metrics, radius, space, text, veil};
use gpui::{
    AnyElement, ClickEvent, CursorStyle, FontWeight, IntoElement, Rgba, SharedString, Window, div,
    prelude::*, px, rgba,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    Primary,
    Outline,
    Ghost,
    Danger,
    DangerGhost,
}

/// The two control heights. A mark-only control is the same shape with its label
/// dropped, so that is a flag on the button rather than two more variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Size {
    /// The default: 28px tall. Toolbars, footers, panel headers.
    Md,
    /// 22px tall. Inside another control's band.
    Sm,
}

impl Size {
    fn height(self) -> f32 {
        match self {
            Size::Md => metrics::CONTROL,
            Size::Sm => metrics::CONTROL_SM,
        }
    }

    fn text(self) -> f32 {
        match self {
            Size::Md => text::SMALL,
            Size::Sm => text::LABEL,
        }
    }

    fn mark(self) -> f32 {
        match self {
            Size::Md => crate::icons::CONTROL,
            Size::Sm => crate::icons::INLINE,
        }
    }

    fn gap(self) -> f32 {
        match self {
            Size::Md => 6.0,
            Size::Sm => space::XS,
        }
    }

    fn pad(self) -> f32 {
        match self {
            Size::Md => space::SM,
            Size::Sm => 6.0,
        }
    }
}

/// The four fill/edge/hover colours a variant needs, resolved from tokens.
struct Skin {
    fill: Rgba,
    text: Rgba,
    edge: Rgba,
    hover_fill: Rgba,
    hover_text: Rgba,
}

fn skin(theme: &Theme, variant: Variant) -> Skin {
    let clear = rgba(0x00000000);
    match variant {
        Variant::Primary => Skin {
            fill: theme.primary,
            text: theme.primary_foreground,
            edge: theme.primary,
            hover_fill: theme.primary_hover,
            hover_text: theme.primary_foreground,
        },
        Variant::Outline => Skin {
            fill: clear,
            text: theme.foreground,
            edge: theme.input,
            hover_fill: theme.band(veil::BAND),
            hover_text: theme.foreground,
        },
        Variant::Ghost => Skin {
            fill: clear,
            text: theme.muted_foreground,
            edge: clear,
            hover_fill: theme.band(veil::BAND),
            hover_text: theme.foreground,
        },
        Variant::Danger => Skin {
            fill: theme.danger_solid,
            text: theme.danger_solid_foreground,
            edge: theme.danger_solid,
            hover_fill: theme.danger,
            hover_text: theme.danger_solid_foreground,
        },
        Variant::DangerGhost => Skin {
            fill: clear,
            text: theme.danger,
            edge: clear,
            hover_fill: theme.danger_soft,
            hover_text: theme.danger,
        },
    }
}

pub struct Button<'a> {
    theme: &'a Theme,
    id: SharedString,
    label: Option<SharedString>,
    mark: Option<Icon>,
    keycap: Option<SharedString>,
    variant: Variant,
    size: Size,
    icon_only: bool,
    enabled: bool,
    tooltip: Option<SharedString>,
}

/// Start a button. See the module docs for which variant to reach for.
pub fn button<'a>(theme: &'a Theme, id: impl Into<SharedString>) -> Button<'a> {
    Button {
        theme,
        id: id.into(),
        label: None,
        mark: None,
        keycap: None,
        variant: Variant::Outline,
        size: Size::Md,
        icon_only: false,
        enabled: true,
        tooltip: None,
    }
}

impl<'a> Button<'a> {
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn icon(mut self, mark: Icon) -> Self {
        self.mark = Some(mark);
        self
    }

    pub fn keycap(mut self, stroke: impl Into<SharedString>) -> Self {
        self.keycap = Some(stroke.into());
        self
    }

    /// A tooltip, shown on hover. Mandatory for a button with no label.
    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip = Some(text.into());
        self
    }

    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    pub fn primary(self) -> Self {
        self.variant(Variant::Primary)
    }

    pub fn outline(self) -> Self {
        self.variant(Variant::Outline)
    }

    pub fn ghost(self) -> Self {
        self.variant(Variant::Ghost)
    }

    pub fn danger(self) -> Self {
        self.variant(Variant::Danger)
    }

    pub fn danger_ghost(self) -> Self {
        self.variant(Variant::DangerGhost)
    }

    pub fn small(mut self) -> Self {
        self.size = Size::Sm;
        self
    }

    /// A button that shows only its mark. Pair it with [`Button::tooltip`].
    pub fn icon_only(mut self) -> Self {
        self.icon_only = true;
        self
    }

    /// Turn the button off without losing its shape. A disabled control is still
    /// read: it keeps its edge and its label, and gives up only its weight.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.enabled = !disabled;
        self
    }

    /// Finish the button and attach its behaviour.
    pub fn build<F>(self, on_click: F) -> AnyElement
    where
        F: Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static,
    {
        let Button {
            theme,
            id,
            label,
            mark,
            keycap,
            variant,
            size,
            icon_only,
            enabled,
            tooltip,
        } = self;

        let skin = skin(theme, variant);
        let height = size.height();

        let mut element = div()
            .id(id)
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .gap(px(size.gap()))
            .h(px(height))
            .flex_shrink_0()
            .rounded(px(radius::CONTROL))
            .font_family(theme.font_ui.clone())
            .text_size(px(size.text()));

        if icon_only {
            element = element.w(px(height));
        } else {
            element = element.px(px(size.pad()));
        }

        if !enabled {
            // Legible, not invisible: the shape and the label stay, the weight goes.
            let fg = theme.muted_foreground;
            let edge = theme.border;
            let mut ghosted = element
                .bg(rgba(0x00000000))
                .text_color(fg)
                .child(mark_of(mark, size, fg));
            if variant != Variant::Ghost && variant != Variant::DangerGhost {
                ghosted = ghosted.border_1().border_color(edge);
            }
            if let Some(label) = label {
                ghosted = ghosted.child(div().child(label));
            }
            if let Some(text) = tooltip {
                ghosted = ghosted.tooltip(crate::components::tooltip::tip(text));
            }
            return ghosted.into_any_element();
        }

        let hover_fill = skin.hover_fill;
        let hover_text = skin.hover_text;

        element = element
            .bg(skin.fill)
            .text_color(skin.text)
            .cursor(CursorStyle::PointingHand)
            .hover(move |style| style.bg(hover_fill).text_color(hover_text))
            .on_click(on_click);

        if variant != Variant::Ghost && variant != Variant::DangerGhost {
            element = element.border_1().border_color(skin.edge);
        }

        if let Some(mark) = mark {
            element = element.child(icon(mark, size.mark(), skin.text));
        }
        if let Some(label) = label {
            element = element.child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .whitespace_nowrap()
                    .child(label),
            );
        }
        if let Some(stroke) = keycap {
            element = element.child(crate::components::kbd(theme, stroke));
        }
        if let Some(text) = tooltip {
            element = element.tooltip(crate::components::tooltip::tip(text));
        }

        element.into_any_element()
    }
}

/// A mark that is present-or-absent, kept in the child list so a labelled button
/// does not shift by 15px between its enabled and disabled states.
fn mark_of(mark: Option<Icon>, size: Size, color: Rgba) -> AnyElement {
    match mark {
        Some(mark) => icon(mark, size.mark(), color).into_any_element(),
        None => div().into_any_element(),
    }
}

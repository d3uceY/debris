//! Design tokens, as a GPUI `Global`.
//!
//! One source of truth for every colour, radius, and step. A leaf component never
//! hardcodes a hex — if a value is not here, it is not part of the design system.
//!
//! Two deliberate departures from the usual GPUI playbook, both required by the
//! Swiss brief and both documented in `DESIGN.md`: corners are square rather than
//! 12–16px, and there are no shadows at all. Separation is a hairline rule.

use gpui::{App, Global, Rgba, SharedString, rgb, rgba};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

/// The 4px ladder. Used at these steps only, so rhythm is a decision rather than a
/// habit.
pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
    pub const XXL: f32 = 32.0;
    pub const XXXL: f32 = 48.0;
}

/// Fixed sizes: desktop at fixed DPI wants fixed steps, not a fluid scale.
pub mod text {
    pub const DISPLAY: f32 = 34.0;
    pub const TITLE: f32 = 20.0;
    pub const SUBTITLE: f32 = 15.0;
    pub const BODY: f32 = 14.0;
    pub const SMALL: f32 = 12.0;
    pub const LABEL: f32 = 11.0;
}

/// Square containers, barely-there controls. See `DESIGN.md`.
pub mod radius {
    pub const CONTROL: f32 = 2.0;
    pub const CONTAINER: f32 = 0.0;
}

/// Structural heights, so the grid is arithmetic rather than guesswork.
pub mod layout {
    pub const HEADER: f32 = 56.0;
    pub const RAIL: f32 = 232.0;
    pub const ROW: f32 = 32.0;
    pub const STATUS_BAR: f32 = 28.0;
    pub const CONTROL: f32 = 32.0;
    pub const CONTENT_MAX: f32 = 1120.0;
    pub const REMOVAL_COLUMN: f32 = 340.0;
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub appearance: Appearance,

    /// The Swiss grotesque. `Arial` is the Helvetica lineage and is present on every
    /// Windows machine; swapping to a bundled `Inter` or `Neue Haas Grotesk` means
    /// changing this one field.
    pub font_ui: SharedString,
    /// Monospace is for data, identifiers, and measurement — which is all of the
    /// text this app shows about packages.
    pub font_mono: SharedString,

    pub surface: Rgba,
    pub surface_sunken: Rgba,
    pub surface_hover: Rgba,
    pub surface_selected: Rgba,

    pub ink: Rgba,
    pub ink_muted: Rgba,
    pub ink_faint: Rgba,
    pub ink_inverse: Rgba,
    pub ink_hover: Rgba,

    pub rule: Rgba,
    pub rule_strong: Rgba,

    pub accent: Rgba,
    pub accent_hover: Rgba,
    pub accent_text: Rgba,
    pub accent_soft: Rgba,

    pub warning: Rgba,
    pub warning_soft: Rgba,
    pub success: Rgba,

    pub focus_ring: Rgba,

    /// GPUI has no `prefers-reduced-motion`. The app exposes a setting and writes it
    /// here, so every animation can consult one flag.
    pub reduce_motion: bool,
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl Theme {
    /// The default. The scene — a deliberate maintenance task at a desk in daylight —
    /// picks it (see PRODUCT.md).
    pub fn light() -> Self {
        Self {
            appearance: Appearance::Light,
            font_ui: "Arial".into(),
            font_mono: "Consolas".into(),

            surface: rgb(0xFFFFFF),
            surface_sunken: rgb(0xF6F6F6),
            surface_hover: rgb(0xF0F0F0),
            surface_selected: rgb(0xE8E8E8),

            ink: rgb(0x111111),
            ink_muted: rgb(0x5C5C5C),
            ink_faint: rgb(0x8A8A8A),
            ink_inverse: rgb(0xFFFFFF),
            ink_hover: rgb(0x000000),

            rule: rgb(0xDFDFDF),
            rule_strong: rgb(0xB4B4B4),

            accent: rgb(0xD92D20),
            accent_hover: rgb(0xB7231A),
            accent_text: rgb(0xFFFFFF),
            accent_soft: rgb(0xFDEDEC),

            warning: rgb(0xB25E09),
            warning_soft: rgb(0xFDF3E7),
            success: rgb(0x1F7A3D),

            focus_ring: rgb(0x111111),
            reduce_motion: false,
        }
    }

    /// The same roles on ink surfaces. The accent lifts so it still clears 4.5:1
    /// against the dark field rather than sitting at the light value.
    pub fn dark() -> Self {
        Self {
            appearance: Appearance::Dark,
            font_ui: "Arial".into(),
            font_mono: "Consolas".into(),

            surface: rgb(0x0E0E0E),
            surface_sunken: rgb(0x171717),
            surface_hover: rgb(0x1F1F1F),
            surface_selected: rgb(0x2A2A2A),

            ink: rgb(0xF2F2F2),
            ink_muted: rgb(0xA8A8A8),
            ink_faint: rgb(0x7A7A7A),
            ink_inverse: rgb(0x0E0E0E),
            ink_hover: rgb(0xFFFFFF),

            rule: rgb(0x2A2A2A),
            rule_strong: rgb(0x4A4A4A),

            accent: rgb(0xFF5A4D),
            accent_hover: rgb(0xFF7A70),
            accent_text: rgb(0x0E0E0E),
            accent_soft: rgb(0x2A1512),

            warning: rgb(0xE0A055),
            warning_soft: rgb(0x2A1E0E),
            success: rgb(0x5FBF7F),

            focus_ring: rgb(0xF2F2F2),
            reduce_motion: false,
        }
    }

    pub fn toggle_appearance(&mut self) {
        let reduce_motion = self.reduce_motion;
        *self = match self.appearance {
            Appearance::Light => Self::dark(),
            Appearance::Dark => Self::light(),
        };
        self.reduce_motion = reduce_motion;
    }

    /// A wash for the accent, used behind a staged row. Kept as a function rather
    /// than a token because it is derived, not chosen.
    pub fn accent_wash(&self) -> Rgba {
        match self.appearance {
            Appearance::Light => rgba(0xD92D200F),
            Appearance::Dark => rgba(0xFF5A4D1F),
        }
    }
}

impl Global for Theme {}

/// `cx.theme()` — shorter than `cx.global::<Theme>()` at every call site.
pub trait ThemeExt {
    fn theme(&self) -> &Theme;
}

impl ThemeExt for App {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}

/// Views are handed a `Context`, so they need the same shorthand. Spelling this out
/// avoids depending on `Context` dereferencing to `App`.
impl<T> ThemeExt for gpui::Context<'_, T> {
    fn theme(&self) -> &Theme {
        self.global::<Theme>()
    }
}

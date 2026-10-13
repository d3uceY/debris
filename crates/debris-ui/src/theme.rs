//! Design tokens, as a GPUI `Global`.
//!
//! One source of truth for every colour, radius, and step. A leaf component never
//! hardcodes a hex — if a value is not here, it is not part of the design system.
//!
//! # The system
//!
//! Tokens are the shadcn/Tailwind **neutral** scale, the same one the reference
//! desktop app uses: a near-monochrome field, one hairline colour, and colour
//! reserved for meaning (removal, caution, success). Depth is never a shadow — it
//! is a token at reduced opacity, laid over the surface beneath it.
//!
//! Read `DESIGN.md` before changing a token. The zone heights and the neutral ramp
//! are the two things every screen is measured against.

use gpui::{App, BoxShadow, Global, Rgba, SharedString, hsla, point, px, rgb};

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
}

/// Fixed sizes: desktop at fixed DPI wants fixed steps, not a fluid scale.
///
/// The top of the ramp is deliberately low. This is a tool, not a landing page —
/// the largest type on a screen is a section title or a count, never a hero.
pub mod text {
    pub const LABEL: f32 = 11.0;
    pub const SMALL: f32 = 12.0;
    pub const BODY: f32 = 13.0;
    pub const SUBTITLE: f32 = 14.0;
}

/// One radius scale, so a chip, a control, a panel, and a rail button are
/// recognisably the same family.
pub mod radius {
    /// Badges and keycaps.
    pub const CHIP: f32 = 5.0;
    /// Buttons, inputs, panels.
    pub const CONTROL: f32 = 6.0;
    /// Icon-rail buttons.
    pub const RAIL: f32 = 8.0;
    /// The icon tile in an empty state.
    pub const TILE: f32 = 10.0;
}

/// Structural heights. The grid is arithmetic, not guesswork — every horizontal
/// band in the shell is one of these.
pub mod metrics {
    /// The window's top band: identity, command shell, global actions.
    pub const TITLE_BAR: f32 = 36.0;
    /// The bottom band: state segments.
    pub const STATUS_BAR: f32 = 26.0;
    /// The title strip on a boxed panel — and the left column's own header, which
    /// shares this value so the bands line up across zones.
    ///
    /// It carries two lines, always: a 14px title over an 11px subtitle. Those two
    /// line boxes measure ~36px, so the height is that plus 4px of clear space above
    /// and below. At 38 there was 1px of it, and the subtitle's descender sat on the
    /// hairline.
    pub const PANEL_HEADER: f32 = 44.0;
    /// A strip of controls between a panel's header and its content. It has to clear
    /// the tallest thing it holds — a `CONTROL` at 28 — with room on both sides; at 30
    /// a button had 1px of breathing space.
    pub const TOOLBAR: f32 = 36.0;
    /// A data table's column header.
    pub const COLUMN_HEADER: f32 = 26.0;
    /// One row of a dense list or table.
    pub const ROW: f32 = 30.0;
    /// A control's height.
    pub const CONTROL: f32 = 28.0;
    /// A control inside another control's band (icon buttons, status chips).
    pub const CONTROL_SM: f32 = 22.0;
    /// The vertical surface switcher.
    pub const ICON_RAIL: f32 = 48.0;
    /// One button on the icon rail.
    pub const RAIL_ITEM: f32 = 34.0;
    /// The left column: the removal queue.
    pub const SIDEBAR: f32 = 304.0;
    /// The right column: the inspector.
    pub const INSPECTOR: f32 = 300.0;
    /// Horizontal inset for a list row or a section header. One number, so a
    /// sidebar's rows and its section labels share an edge.
    pub const INSET: f32 = 8.0;
    /// The command shell in the title bar.
    pub const COMMAND_SHELL: f32 = 420.0;
}

/// The opacity ladder for *depth without shadow*.
///
/// GPUI gives you flat fills; it does not give you a material. So a header strip, a
/// toolbar, and a hovered row are all the same `muted` token, laid over the surface
/// beneath at one of these fractions. Naming them stops the drift you get from
/// sprinkling `0.2`, `0.22`, `0.25` through a codebase.
pub mod veil {
    /// A header or toolbar band. Barely there; it separates without a border.
    pub const BAND: f32 = 0.22;
    /// A quieter band — a strip inside an already-tinted region.
    pub const BAND_QUIET: f32 = 0.14;
    /// A chip, a badge, a pill's own fill.
    pub const CHIP: f32 = 0.34;
    /// A panel's own border, one step softer than the structural rule.
    pub const HAIRLINE: f32 = 0.72;
    /// A table's row rule. Softer still, because there are many of them.
    pub const ROW_RULE: f32 = 0.55;
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub appearance: Appearance,

    /// The UI face. `Segoe UI` is the platform's own grotesque and is present on
    /// every Windows machine; nothing is bundled, so this is the one field to
    /// change to move the whole app onto a downloaded face.
    pub font_ui: SharedString,
    /// Monospace is for data, identifiers, and measurement — which is most of the
    /// text this app shows about packages.
    pub font_mono: SharedString,

    /// The page: the center dock and the inspector.
    pub background: Rgba,
    /// The title bar band.
    pub title_bar: Rgba,
    /// The icon rail and the left column.
    pub sidebar: Rgba,
    /// A selected rail button, or a selected row in the left column. The one
    /// "this is where you are" fill in the shell.
    pub sidebar_accent: Rgba,

    /// Text at full strength.
    pub foreground: Rgba,
    /// Text at second strength. The only other text colour there is — every value
    /// in this interface is either `foreground` or this, and both clear 4.5:1 on
    /// every surface they are allowed to sit on.
    pub muted_foreground: Rgba,

    /// The structural hairline: panel edges, column edges, band edges.
    pub border: Rgba,
    /// The edge of the left column, one step darker than a panel's.
    pub sidebar_border: Rgba,
    /// The resting edge of an input, which sits a step *above* `border` so a field
    /// reads as a field rather than a slot.
    pub input: Rgba,

    /// The ordinary hover fill, for rows and quiet buttons.
    pub muted: Rgba,

    /// The one primary action on a screen.
    pub primary: Rgba,
    pub primary_foreground: Rgba,
    pub primary_hover: Rgba,
    /// A neutral selected/hovered fill one step stronger than `muted`.
    pub accent: Rgba,

    /// Removal, as text and as a border. Nothing else is red.
    pub danger: Rgba,
    /// Red as a *fill*, with its own readable foreground.
    pub danger_solid: Rgba,
    pub danger_solid_foreground: Rgba,
    /// A wash of red for a staged row.
    pub danger_soft: Rgba,

    /// Caution: a system-critical package, a dry run that is switched off.
    pub warning: Rgba,
    pub warning_soft: Rgba,

    /// Confirmed.
    pub success: Rgba,
    pub success_soft: Rgba,

    /// The keyboard focus ring.
    pub ring: Rgba,
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

impl Theme {
    pub fn light() -> Self {
        Self {
            appearance: Appearance::Light,
            font_ui: "Segoe UI".into(),
            font_mono: "Consolas".into(),

            background: rgb(0xFFFFFF),
            title_bar: rgb(0xF8F8F8),
            sidebar: rgb(0xFAFAFA),
            sidebar_accent: rgb(0xE5E5E5),

            foreground: rgb(0x0A0A0A),
            muted_foreground: rgb(0x737373),

            border: rgb(0xE5E5E5),
            sidebar_border: rgb(0xE5E5E5),
            input: rgb(0xD4D4D4),

            muted: rgb(0xF5F5F5),

            primary: rgb(0x171717),
            primary_foreground: rgb(0xFAFAFA),
            primary_hover: rgb(0x404040),
            accent: rgb(0xE5E5E5),

            danger: rgb(0xDC2626),
            danger_solid: rgb(0xDC2626),
            danger_solid_foreground: rgb(0xFAFAFA),
            danger_soft: rgb(0xFEF2F2),

            warning: rgb(0xB45309),
            warning_soft: rgb(0xFFFBEB),

            success: rgb(0x15803D),
            success_soft: rgb(0xF0FDF4),

            ring: rgb(0x171717),
        }
    }

    /// The same roles on ink surfaces. Every semantic lifts so it still clears
    /// 4.5:1 against the dark field rather than sitting at the light value — a dark
    /// theme is a re-derivation, not an inversion.
    pub fn dark() -> Self {
        Self {
            appearance: Appearance::Dark,
            font_ui: "Segoe UI".into(),
            font_mono: "Consolas".into(),

            background: rgb(0x0A0A0A),
            title_bar: rgb(0x171717),
            sidebar: rgb(0x0A0A0A),
            sidebar_accent: rgb(0x262626),

            foreground: rgb(0xFAFAFA),
            muted_foreground: rgb(0xA3A3A3),

            border: rgb(0x262626),
            sidebar_border: rgb(0x262626),
            input: rgb(0x2F2F2F),

            muted: rgb(0x262626),

            primary: rgb(0xFAFAFA),
            primary_foreground: rgb(0x171717),
            primary_hover: rgb(0xD4D4D4),
            accent: rgb(0x333333),

            danger: rgb(0xF87171),
            danger_solid: rgb(0xDC2626),
            danger_solid_foreground: rgb(0xFAFAFA),
            danger_soft: rgb(0x2A1210),

            warning: rgb(0xFBBF24),
            warning_soft: rgb(0x2A1F08),

            success: rgb(0x4ADE80),
            success_soft: rgb(0x0B2313),

            ring: rgb(0xFAFAFA),
        }
    }

    pub fn toggle_appearance(&mut self) {
        *self = match self.appearance {
            Appearance::Light => Self::dark(),
            Appearance::Dark => Self::light(),
        };
    }

    /// A wash of red for a staged row. Deliberately strong enough to read at a
    /// glance against the sidebar's own off-white — the row has to look *marked*,
    /// not merely tinted.
    pub fn danger_wash(&self) -> Rgba {
        match self.appearance {
            Appearance::Light => rgb(0xDC2626).opacity(0.10),
            Appearance::Dark => rgb(0xF87171).opacity(0.14),
        }
    }

    /// The band fill behind a panel header, a toolbar, or a column header.
    pub fn band(&self, veil: f32) -> Rgba {
        self.muted.opacity(veil)
    }

    /// The wash behind a floating dialog. Ink over whatever is beneath, so the surface
    /// under it reads as inert while the dialog stays a plain panel on top — no blur,
    /// no glass, just the same depth system the rest of the app uses.
    pub fn scrim(&self) -> Rgba {
        match self.appearance {
            Appearance::Light => rgb(0x0A0A0A).opacity(0.32),
            Appearance::Dark => rgb(0x000000).opacity(0.55),
        }
    }

    /// The shadow under a floating surface. This is the second shadow the app is
    /// allowed, and it exists for the same reason as the tooltip's: a dialog genuinely
    /// floats over arbitrary content, and a hairline alone cannot separate it.
    pub fn overlay_shadow(&self) -> Vec<BoxShadow> {
        let color = match self.appearance {
            Appearance::Light => hsla(0.0, 0.0, 0.0, 0.18),
            Appearance::Dark => hsla(0.0, 0.0, 0.0, 0.50),
        };
        vec![BoxShadow {
            color,
            offset: point(px(0.0), px(10.0)),
            blur_radius: px(30.0),
            spread_radius: px(0.0),
            inset: false,
        }]
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

//! The icon set.
//!
//! Icons are drawn as filled SVG paths on a 16px grid, in one visual language and
//! one weight. No emoji and no font glyphs standing in for a drawing.
//!
//! GPUI tints an SVG from the text colour in scope (`Svg::paint` reads
//! `style.text.color`), so an icon inherits the surrounding text colour and takes
//! the accent or the muted ink for free. Set the colour on the parent.

use gpui::{Styled, Svg, px, svg};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    /// The package inventory.
    Package,
    /// The PowerShell library.
    Script,
    /// Saved presets.
    Layers,
    /// The search field.
    Search,
    /// A completed step.
    Check,
    /// A caveat in the removal plan.
    Warning,
    /// Stage — the row leaves the inventory.
    ArrowRight,
    /// Unstage — the row returns.
    ArrowLeft,
    /// Safety and elevation.
    Shield,
}

impl Icon {
    /// Paths are relative to the asset root the app registers.
    pub fn path(self) -> &'static str {
        match self {
            Icon::Package => "icons/package.svg",
            Icon::Script => "icons/script.svg",
            Icon::Layers => "icons/layers.svg",
            Icon::Search => "icons/search.svg",
            Icon::Check => "icons/check.svg",
            Icon::Warning => "icons/warning.svg",
            Icon::ArrowRight => "icons/arrow-right.svg",
            Icon::ArrowLeft => "icons/arrow-left.svg",
            Icon::Shield => "icons/shield.svg",
        }
    }

    /// The name of the SVG file without its directory or extension — used for
    /// element ids, where uniqueness matters more than the path.
    pub fn slug(self) -> &'static str {
        match self {
            Icon::Package => "package",
            Icon::Script => "script",
            Icon::Layers => "layers",
            Icon::Search => "search",
            Icon::Check => "check",
            Icon::Warning => "warning",
            Icon::ArrowRight => "arrow-right",
            Icon::ArrowLeft => "arrow-left",
            Icon::Shield => "shield",
        }
    }
}

/// An icon at `size`, tinted by the text colour in scope.
///
/// Returns the concrete `Svg` rather than `impl IntoElement` so callers can keep
/// styling it — `icon(..).text_color(..)` is the common case.
pub fn icon(icon: Icon, size: f32) -> Svg {
    svg()
        .path(icon.path())
        .w(px(size))
        .h(px(size))
        // An icon in a flex row must never be the thing that gets squeezed.
        .flex_shrink_0()
}

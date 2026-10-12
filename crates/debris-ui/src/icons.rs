//! The icon set.
//!
//! # How these work
//!
//! GPUI does not paint an SVG the way a browser does: it rasterises the file to an
//! **alpha mask** and fills that mask with a colour (see
//! [`SvgRenderer::render_alpha_mask`]). The drawing's own colours are irrelevant —
//! only its alpha survives.
//!
//! Two consequences shape this set:
//!
//! * Every file is a stroked outline on a 24-unit grid with `fill="none"` and
//!   `stroke="#000"`. The `#000` is arbitrary; it is the *shape* that is kept.
//! * The colour comes from `style.text.color` — which, in this revision of GPUI, is
//!   **not inherited**. `Interactivity::paint` computes a style from the element's
//!   own refinement alone and hands *that* to the painter; the inherited text style
//!   lives in a separate window-level stack that only text elements read. So an
//!   `Svg` inside a `text_color(..)` parent has `text.color == None`, and
//!   `Svg::paint` does nothing at all — a silently invisible icon.
//!
//! Therefore [`icon`] takes its colour as a required argument. Passing it at the call
//! site is not ceremony; it is the only thing that works.
//!
//! [`SvgRenderer::render_alpha_mask`]: https://docs.rs/resvg

use gpui::{Rgba, Styled, Svg, px, svg};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    // — Surfaces —
    /// The package inventory.
    Package,
    /// The PowerShell library.
    Terminal,
    /// Saved presets.
    Layers,

    // — Chrome —
    Search,
    /// Toggle the left column.
    PanelLeft,
    /// Toggle the inspector.
    PanelRight,
    Sun,
    Moon,
    Settings,
    Refresh,
    More,

    // — Actions —
    Plus,
    Close,
    Check,
    ArrowRight,
    ArrowLeft,
    ChevronDown,
    ChevronRight,
    Trash,
    Filter,
    List,

    // — Meaning —
    Alert,
    Info,
    Shield,
    Store,
    Cpu,
    Clock,
    Eye,
}

impl Icon {
    /// Paths are relative to the asset root the app registers.
    pub fn path(self) -> &'static str {
        match self {
            Icon::Package => "icons/package.svg",
            Icon::Terminal => "icons/terminal.svg",
            Icon::Layers => "icons/layers.svg",
            Icon::Search => "icons/search.svg",
            Icon::PanelLeft => "icons/panel-left.svg",
            Icon::PanelRight => "icons/panel-right.svg",
            Icon::Sun => "icons/sun.svg",
            Icon::Moon => "icons/moon.svg",
            Icon::Settings => "icons/settings.svg",
            Icon::Refresh => "icons/refresh.svg",
            Icon::More => "icons/more.svg",
            Icon::Plus => "icons/plus.svg",
            Icon::Close => "icons/x.svg",
            Icon::Check => "icons/check.svg",
            Icon::ArrowRight => "icons/arrow-right.svg",
            Icon::ArrowLeft => "icons/arrow-left.svg",
            Icon::ChevronDown => "icons/chevron-down.svg",
            Icon::ChevronRight => "icons/chevron-right.svg",
            Icon::Trash => "icons/trash.svg",
            Icon::Filter => "icons/filter.svg",
            Icon::List => "icons/list.svg",
            Icon::Alert => "icons/alert.svg",
            Icon::Info => "icons/info.svg",
            Icon::Shield => "icons/shield.svg",
            Icon::Store => "icons/store.svg",
            Icon::Cpu => "icons/cpu.svg",
            Icon::Clock => "icons/clock.svg",
            Icon::Eye => "icons/eye.svg",
        }
    }

    /// The file stem — used for element ids, where uniqueness matters more than the
    /// path.
    pub fn slug(self) -> &'static str {
        match self {
            Icon::Package => "package",
            Icon::Terminal => "terminal",
            Icon::Layers => "layers",
            Icon::Search => "search",
            Icon::PanelLeft => "panel-left",
            Icon::PanelRight => "panel-right",
            Icon::Sun => "sun",
            Icon::Moon => "moon",
            Icon::Settings => "settings",
            Icon::Refresh => "refresh",
            Icon::More => "more",
            Icon::Plus => "plus",
            Icon::Close => "x",
            Icon::Check => "check",
            Icon::ArrowRight => "arrow-right",
            Icon::ArrowLeft => "arrow-left",
            Icon::ChevronDown => "chevron-down",
            Icon::ChevronRight => "chevron-right",
            Icon::Trash => "trash",
            Icon::Filter => "filter",
            Icon::List => "list",
            Icon::Alert => "alert",
            Icon::Info => "info",
            Icon::Shield => "shield",
            Icon::Store => "store",
            Icon::Cpu => "cpu",
            Icon::Clock => "clock",
            Icon::Eye => "eye",
        }
    }
}

/// An icon at `size`, painted in `color`.
///
/// Returns the concrete `Svg` rather than `impl IntoElement` so callers can keep
/// styling it (padding, opacity). The colour is a parameter rather than something
/// inherited — see the module docs; `Svg` does not inherit `text_color`, and an
/// `Svg` with no colour paints nothing.
///
/// The 24-unit drawing and the requested box rarely divide evenly (`14 / 24`), and
/// that is intentional: the grid is a drawing convention, not a pixel constraint.
/// Strokes land on half-pixels and the rasteriser smooths them.
pub fn icon(mark: Icon, size: f32, color: Rgba) -> Svg {
    svg()
        .path(mark.path())
        .w(px(size))
        .h(px(size))
        // An icon in a flex row must never be the thing that gets squeezed.
        .flex_shrink_0()
        .text_color(color)
}

/// The standard inline size for an icon sitting in text or a row.
pub const INLINE: f32 = 14.0;
/// The size for an icon in a toolbar or a control.
pub const CONTROL: f32 = 15.0;
/// The size for an icon that is the only content of a control.
pub const BUTTON: f32 = 16.0;
/// The size for the mark in an empty state's tile.
pub const TILE: f32 = 18.0;
/// The size for an icon on the rail, where it is the primary content.
pub const RAIL: f32 = 18.0;

# DESIGN.md — debris

## World

```
BRAND ESSENCE:     exacting
VISUAL TENSION:    clinical white expanse ↔ one signal-red accent;
                   dense monospaced data ↔ generous margins
SIGNATURE MOMENT:  the staging ledger — a row leaves the inventory column and
                   arrives in the removal column across a hairline rule, while
                   the count ticks over in tabular figures
FRAMEWORK AMBITION: a virtualized inventory (`uniform_list`) that holds its frame
                   at several thousand rows, and an SVG icon set tinted from one
                   ink token
```

**Stance: light.** The scene decides it (see PRODUCT.md) — a deliberate maintenance
task at a desk in daylight, lasting minutes. Dark tokens exist and the components
read from tokens only, so the appearance is switchable, but light is the default.

**Mode: Operate.** This is a task surface. Familiarity is a feature. The tool should
disappear into the job; expression is spent on the ledger and on nothing else.

## The Swiss deviation — read this before "fixing" it

The craft floor's defaults for raised surfaces (12–16px radii, soft shadows, cards)
are **deliberately not used**. The brief is Swiss/International Typographic Style:
structure comes from the grid, the rule, and the type — never from a container's
padding and shadow. So, in this project:

- **Radii are ~0.** Containers are square (`rounded_none`). Controls get 2px. Pills are never used.
- **There are no shadows.** Elevation is not a thing here; separation is a hairline rule and whitespace.
- **Declare structure once.** A 1px rule `or` a background change — never a border *and* a shadow *and* a background tint.
- **The one accent is red and it means "this will be removed / this is dangerous."** It is never decorative. Nothing else in the interface is colored.

## Type

| Role | Family | Notes |
|---|---|---|
| Display + UI | **Arial** (Helvetica lineage) | The Swiss grotesque. `Neue Haas Grotesk`, `Inter`, or `Helvetica Now` are drop-in upgrades — change `Theme::font_ui` only. |
| Data + identifiers | **Consolas** | Package names, versions, counts, paths. Monospace is for data and measurement, which is exactly what this is. |

Scale (fixed px — desktop at fixed DPI):

| Token | Size | Use |
|---|---|---|
| `display` | 34 | The one headline per surface |
| `title` | 20 | Section headings |
| `body` | 14 | Everything |
| `small` | 12 | Metadata, row secondaries |
| `label` | 11 | Uppercase section labels, `letter_spacing` widened |

Rules: flush left, ragged right — never centered body text. Type carries hierarchy,
so headings get weight and space, not a colored box. Tabular figures for every count
that changes.

> **GPUI limitation, worth knowing before you plan tracking.** `TextStyle` has no
> `letter_spacing`, so the widened tracking this style usually wants on uppercase
> labels is not available. Uppercase labels are set small and in `SEMIBOLD` instead,
> which separates them from body text on weight rather than on tracking. Do not
> specify tracking in a spec you cannot implement.

## Color

Derived from one ink hue on a true white field. Light:

| Token | Value | Role |
|---|---|---|
| `surface` | `#FFFFFF` | The page |
| `surface_sunken` | `#F6F6F6` | The staging column, the sidebar |
| `surface_hover` | `#F0F0F0` | Row hover |
| `surface_selected` | `#E8E8E8` | Row selected |
| `ink` | `#111111` | Primary text |
| `ink_muted` | `#5C5C5C` | Secondary (4.5:1+ on white) |
| `ink_faint` | `#8A8A8A` | Tertiary, non-essential only |
| `rule` | `#DFDFDF` | Hairlines — the only separator |
| `rule_strong` | `#B4B4B4` | Column edges |
| `accent` | `#D92D20` | Removal / danger. Nothing else. |
| `accent_text` | `#FFFFFF` | Text on accent |
| `warning` | `#B25E09` | Caution risk |
| `success` | `#1F7A3D` | Completed |

Dark mirrors the same roles on ink surfaces (`#0E0E0E` page, `#F2F2F2` ink); the
accent lifts to `#FF5A4D` to hold ≥ 4.5:1 against the dark surface.

Contrast is computed from these values, not eyeballed. Nothing assumes white.

## Space

A 4px ladder, used at these steps only: `4 8 12 16 24 32 48 64`. Gaps between
siblings, never stacked margins. More space above a heading than below it. Dense
inside rows (12px vertical), generous between regions (32–48px).

Grid: a fixed 232px rail and a fluid content column, content capped at 1120px,
48px gutters. Structure is visible as hairlines, not as boxes.

## Components

- **`Rule`** — the 1px hairline. The primary structural device in this interface.
- **`Button`** — square, 2px radius. Variants: `Default` (ink rule, transparent), `Primary` (ink fill), `Danger` (accent fill), `Quiet` (no rule until hover).
- **`PackageRow`** — mono name, muted publisher, 32px tall, hairline beneath, hover + selected states, keyboard focus ring.
- **`Stat`** — a tabular count over an uppercase label. The only numbers that get size.
- **`RiskTag`** — uppercase 11px, category-colored. The only place color appears besides removal.
- **`StatusBar`** — the bottom edge: what just happened, in one line, never a toast.
- **`Icon`** — 16px SVG, tinted from the surrounding text color. One stroke weight, one visual language, no unicode-as-icon.

## Motion

**One authored moment per surface**, not a fade on everything:

- The ledger transfer: the row's background resolves to accent-tinted and the count
  ticks — `120ms`, `ease_out_quint`.
- No motion on hover or scroll. Instant reads as precise; latency reads as sloppy.

Everything is gated on a `reduce_motion` flag in the theme, because GPUI has no
`prefers-reduced-motion`. Non-essential motion stops when it is set; state changes
still read through color and rule weight.

## States

Every data surface designs: `Loading` (a rule-width determinate bar, never a spinner),
`Empty` (teaches what the surface is for), `Error` (names the problem and the recovery,
in the product's own voice), `Denied` (not elevated, with the way to become elevated).
Disabled controls stay legible rather than fading to invisibility.

Coverage: no target below 28px; the keyboard reaches every action the mouse can.

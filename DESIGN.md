# DESIGN.md — debris

## World

```
BRAND ESSENCE:     instrument
VISUAL TENSION:    near-monochrome tool surface ↔ one red signal that means "this is leaving"
SIGNATURE MOMENT:  the removal queue — a row crosses the hairline out of the table
                   into the left column, the mark becomes a check, the wash goes red,
                   and the count in the column header ticks over
FRAMEWORK AMBITION: a four-zone shell where each zone owns its own scroll, three of
                   them bands at exactly the same height, and the table virtualizes
                   to five thousand rows without the frame noticing
```

**Stance: both, light is the default.** The scene is a desk in daylight, so light leads
— but the dark set is derived from the same roles, not inverted from the light one,
and every screen is built from tokens only. Both are first-class; neither is a theme
someone bolted on.

**Mode: Operate.** This is a task surface. The reference point is a database client, not
a landing page: dense rows, mono data, bands that align, and a shell the eye stops
noticing after ten seconds. Expression is spent on the queue and on the inspector —
the two places where the user has to *understand* something — and nowhere else.

## The lineage — read this before "fixing" it

The visual language is the shadcn/Tailwind **neutral** scale, as used by the desktop
client this shell is modelled on. Three consequences, all deliberate:

- **Depth is opacity, not shadow.** There is exactly one shadow in the app, under the
  tooltip, and it exists because a tooltip floats over arbitrary content. Everything
  else separates by laying one token over another at a named fraction: a header band,
  a toolbar, a hovered row, a chip and a table rule are all `muted` at different
  values. See [The veil ladder](#the-veil-ladder).
- **Radii are a scale, not a style.** 5 for a chip, 6 for a control or an input, 8 for
  a rail button, 10 for an empty state's tile. Containers are not rounded.
- **Colour is a vocabulary, not decoration.** Publishers, versions, counts and labels
  are all grey. Red means removal. Amber means "Windows needs this". Green means done.
  If a new colour appears in a leaf component without a semantic reason, that is the
  bug.

## The shell

Four zones, three of them toggleable, on a 36px / 26px sandwich:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│ 36   mark · surface        [ search ]        appearance · panels · Rescan    │
├──────┬──────────────────────────┬──────────────────────────┬─────────────────┤
│ 48   │ 304                      │ center dock              │ 300             │
│ rail │ removal queue            │ 38 header                │ inspector       │
│      │ 38 header                │ 30 toolbar               │ 38 header       │
│      │ rows                     │ 26 column header         │ groups          │
│      │ footer (the confirm)     │ rows                     │                 │
├──────┴──────────────────────────┴──────────────────────────┴─────────────────┤
│ 26  ● what just happened        │ elevation │ motion                          │
└──────────────────────────────────────────────────────────────────────────────┘
```

**Rules that hold it together**

1. **Each zone owns its own scroll.** The rail, the queue, the table and the inspector
   are four independent scroll regions. The shell itself never scrolls; `min_h_0` on
   every flex ancestor is what makes that true rather than aspirational.
2. **Bands align.** The queue header, the panel header, and the inspector header are all
   38px on the same hairline. That alignment is the entire reason four regions read as
   one window instead of four panels.
3. **One search field.** The filter lives in the title bar and applies to whichever
   surface is up. A second, per-screen copy of the same field is how a shell starts
   contradicting itself.
4. **The queue is not screen-dependent.** Staging is the only state that outlives the
   surface you are standing on. Putting it behind a tab means a preset loaded on one
   screen silently scrolls out of sight on another.
5. **The inspector is off by default.** It is a second opinion, not the main event. The
   rail and the status bar are where you turn it on.

## Type

| Role | Family | Notes |
|---|---|---|
| Display + UI | **Segoe UI** | The platform's own grotesque, present on every Windows machine. Nothing is bundled, so `Theme::font_ui` is the single field to change to move onto a downloaded face. |
| Data + identifiers | **Consolas** | Package names, versions, counts, keys, paths. Every identity this app shows is an identifier, so mono is the default for data rather than a flourish. |

Scale (fixed px — desktop at fixed DPI):

| Token | Size | Use |
|---|---|---|
| `subtitle` | 14 | A panel title, a row title, the confirm line |
| `body` | 13 | The base: row text, prose |
| `small` | 12 | Table cells, captions, footnotes |
| `label` | 11 | Chips, section eyebrows, column headers, the status rail |

**There is no display size.** This is the deliberate departure from the previous
revision of this file, which set 34px headlines over each surface. A tool that shouts its
own name at you on every screen is spending the top of its type scale on nothing. The
largest type in the app is a 14px panel title or a 14px confirm line; hierarchy comes
from weight, colour, and *position* instead.

Rules: flush left, ragged right. Every count that changes is mono, so it does not
jitter as it ticks. Nothing is centred except an empty state and a status message.

## Colour

The neutral ramp, light → dark:

| Token | Light | Dark | Role |
|---|---|---|---|
| `background` | `#FFFFFF` | `#0A0A0A` | The page: center dock, inspector |
| `title_bar` | `#F8F8F8` | `#171717` | The top band |
| `sidebar` | `#FAFAFA` | `#0A0A0A` | The rail and the queue |
| `sidebar_accent` | `#E5E5E5` | `#262626` | The selected rail button, the selected nav row |
| `foreground` | `#0A0A0A` | `#FAFAFA` | Text |
| `muted_foreground` | `#737373` | `#A3A3A3` | Everything secondary |
| `border` | `#E5E5E5` | `#262626` | Structural hairlines |
| `input` | `#D4D4D4` | `#2F2F2F` | A control's resting edge |
| `muted` | `#F5F5F5` | `#262626` | The one band/hover fill |
| `primary` | `#171717` | `#FAFAFA` | The one committing action |
| `danger` | `#DC2626` | `#F87171` | Removal, as text |
| `warning` | `#B45309` | `#FBBF24` | System-critical |
| `success` | `#15803D` | `#4ADE80` | Confirmed |

Two text colours, not three. The previous revision carried a third "faint" grey that
measured 2.5:1 on white — used for real labels, which made them unreadable. Anything
that needs to read is `foreground` or `muted_foreground`, and both clear 4.5:1 on every
surface they are allowed to sit on. Opacity is for *fills, rules and icon strokes* — the
things nobody has to read.

### The veil ladder

Colours never appear as one-offs. `veil::*` names the fractions so that a header band in
the queue and a header band in the dock are the same value by construction:

| Name | Value | Used for |
|---|---|---|
| `BAND` | 0.22 | The toolbar, the icon rail's hover, a ghost button's hover |
| `BAND_QUIET` | 0.14 | A panel header, a column header — the quieter top strip |
| `CHIP` | 0.34 | A chip's own fill, an empty state's tile |
| `HAIRLINE` | 0.72 | A panel's own edge, one step softer than `border` |
| `ROW_RULE` | 0.55 | The rule between rows, because there are many of them |
| `ICON` | 0.9 | A decorative icon's stroke |

## Space and geometry

A 4px ladder, used at these steps only: `4 8 12 16 24 32`.

Structural heights are constants, not judgement calls. `theme::metrics` is the list, and
the numbers that matter are `TITLE_BAR 36`, `PANEL_HEADER 38`, `TOOLBAR 30`,
`COLUMN_HEADER 26`, `ROW 30`, `CONTROL 28`, `ICON_RAIL 48`, `SIDEBAR 304`,
`INSPECTOR 300`, `STATUS_BAR 26`.

The panel header is 38 because it carries **two lines, always**: a title and a subtitle.
A header that can only fit one line forces every panel to choose between naming itself
and saying anything useful — which is how you end up with a screen called "Packages" and
no idea how many there are.

## The seven shapes

There are seven shapes in this interface and nothing else. If a screen needs an eighth,
that is a design decision, not an implementation detail.

- **Band** — a full-width tinted strip separating two regions. The only depth device.
- **Rule** — hairline separation inside a region. Not a border on a container: a list is
  a stack of rules, not a card with padding.
- **Row** — the unit of every list. Four slots: a fixed **lead** (mark), a flexible
  **name**, fixed **meta** columns, and a `flex_shrink_0` **trailing** slot. A row
  without a mark still reserves the lead slot so labels align down the column. Rows are
  `w_full` — without it a row is content-sized and every fixed column drifts.
- **Chip** — a badge, a keycap, a count. 18px tall, 5px radius, 11px type.
- **Panel header** — a title strip. A title *and* a subtitle, always.
- **Empty state** — a 42px tile with a mark, a title, a sentence, optional action. The
  sentence teaches what the surface is for; it never apologises.
- **Control** — button, field, switch. Four button variants: `Primary` (the one
  committing action), `Outline`, `Ghost`, `Danger`; plus `DangerGhost` for a destructive
  escape.

Two shapes that are deliberately **not** used: the card (a row plus a rule is enough)
and the modal (the confirm lives in the queue's footer, where it stays readable instead
of being dismissed).

## Icons

Every icon is a stroked outline on a 24-unit grid: `fill="none"`, `stroke="#000"`,
`stroke-width="2"`, round caps and joins. They live in `assets/icons/` and are resolved
through the app's `AssetSource`.

**GPUI paints an SVG as an alpha mask** — it rasterises the file and fills the mask with
a colour. The drawing's own colours are irrelevant; only its alpha survives. And that
colour is **not inherited**: `Interactivity::paint` computes an element's style from its
own refinement alone and hands *that* to the painter, while the inherited text style sits
in a separate window-level stack that only text elements read. An `Svg` inside a
`text_color(..)` parent has `text.color == None`, and `Svg::paint` then does nothing at
all — a silently invisible icon, with no error anywhere.

So `icons::icon(mark, size, color)` takes the colour as a required argument. That is not
ceremony; it is the only thing that works. Any new icon call site passes a colour.

## Motion

**One authored moment, nothing else.** The queue is the moment: a row arrives with the
removal wash and the header count changes. Everything else is instant, because instant
reads as precise and latency reads as sloppy.

The only two animations in the app are the indeterminate loading bar (a rule that pulses
between 0.2 and 1.0 opacity) and the busy dot in the status rail. Both are gated on
`Theme::reduce_motion`, because GPUI has no `prefers-reduced-motion`; the app exposes a
setting and the theme carries it.

There are no element transforms to animate with. GPUI's styling API has no `transform`,
so the switch's knob travels by changing the row's justification, not by translating —
the state change is instant and the *colour* carries the feedback.

## States

Every data surface designs all of them:

- **Loading** — a rule-width bar under the column header, never a spinner in the middle
  of content that is about to arrive.
- **Empty** — four different nothings, four different sentences: not scanned yet,
  filtered to nothing, everything already staged, and the library genuinely absent. "No
  results" is only truthful once you know *which* nothing it is.
- **Error** — names the problem and the recovery, in the product's own voice, with the
  raw PowerShell error quoted rather than paraphrased.
- **Disabled** — stays legible. A disabled control keeps its edge and its label and
  gives up only its weight.
- **Keyboard** — the table has a cursor, `↑`/`↓` move it, `Enter` stages. The inspector's
  shortcut list only makes promises the table keeps.

## Coverage audit

No target below 22px; every interactive target is at least 28px except the icon buttons
in the title bar and the field's clear affordance, which are 22 and 16. Contrast is
computed from the token values above, not eyeballed. Light and dark are both rendered
and inspected; nothing assumes a light background.

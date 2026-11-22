# Layout Strategies

`apply_layout` arranges cards through a `LayoutStrategy<Pixels>` —
the same contract `smearor-commons-layout` defines for headless
tabletop layouting. The board maps cards to `LayoutItem`s, calls
`arrange` against the workspace bounds and applies every `Placement`
through the regular geometry path.

```rust,ignore
use smearor_commons_layout::{FlowLayout, SeatLayout};

// All visible cards, gliding animatedly:
board.apply_layout(&FlowLayout::default(), None, true)?;

// Only a selection, snapping:
board.apply_layout(&FlowLayout::default(), Some(&[id_a, id_b]), false)?;

// Around the table edges:
board.apply_layout(&SeatLayout::default(), None, true)?;
```

## What the board hands the strategy

Each selected card becomes a `LayoutItem` carrying its **displayed
footprint** (natural size under current scale, rotated), its live
rotation and its `seat`. The strategy sees footprints, never widgets
— it answers `Placement`s with a target center, size, rotation and
optionally a seat.

`Placement::size` is the target footprint; cards keep their
configured `scale_limits`. Because placements flow through the normal
geometry path, clamping, scale limits and `geometry-changed` behave
exactly like manual manipulation — and with `animated: true` every
placement becomes a per-card frame-clock glide.

## `DoesNotFit`

A strategy reports the first item it cannot place
(`LayoutError::DoesNotFit { index, items }`), surfaced as
`BoardError::Layout(...)`. A failed arrange **leaves the board
untouched** — placements are computed in full before any is applied,
so a layout either lands completely or not at all.

## Built-in and custom

| Strategy | Behavior |
|---|---|
| `FlowLayout` | anchored rows (`AnchorWeights` corner + `gap`), non-overlapping, `wrap` to the next line |
| `SeatLayout` | items along table edges — pinned seats take their offset, unpinned spread centered, rotated to the side's orientation |
| custom | any `LayoutStrategy<Pixels>` impl — signatures in the [layout book](https://smearor.github.io/smearor-commons/book/layout/strategies.html) |

Sticky cards are skipped entirely — strategies arrange table
content, not screen chrome.

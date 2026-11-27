# Domain Model

The `floating_board::domain` module is the heart of the library. It contains
**no `gtk4` dependency**: only newtypes, enums and pure geometry functions.
Everything here is display-free and unit-testable — the same types appear in
the widget API, in gesture handling, and in the persisted layout files.

## Design Principles

1. **Radically strict types.** Every concept is a newtype or enum with
   semantic meaning: a `CardPosition` is never confused with a pixel offset,
   and a `ScaleFactor` can never silently become negative.
2. **The board owns the state.** Position, rotation and scale exist exactly
   once per card — inside the `FloatingBoard`. `Card` objects are views that
   delegate every geometry read and write to the board. Widget children never
   own geometry state.
3. **Feature-gated serialization.** Serde implementations on the domain types
   live behind the `serde` cargo feature (enabled by default). Building with
   `--no-default-features` removes the persistence dependency entirely.

## Type Reference

### Identity

| Type | Description |
|------|-------------|
| `CardId` | Newtype over `String`. Consumers assign stable, semantic keys such as `"controls"` or `"params"` that survive in persisted layouts. `add_card` generates `card-N` ids; `add_card_with_id` takes a semantic key. |
| `CardPosition` | Newtype over `(f64, f64)` workspace pixels — the card's **center point**, not its top-left corner. The center anchor stays stable under rotation and scaling. Serializes as a `[x, y]` pair. (Sticky cards are the exception: while pinned they store widget pixels.) |

### Geometry

| Type | Description |
|------|-------------|
| `CardGeometry` | `{ position: CardPosition, rotation: SmearorRotation, scale: ScaleFactor }` — the complete geometric state of a card and the unit of persistence. |
| `ScaleFactor` | Newtype over `f64` (`1.0` = natural size). Always strictly positive; per-card limits are applied via `ScaleBounds`. |
| `ScaleBounds` | `{ min: f64, max: f64 }` — inclusive scale limits, default `0.5 – 2.0`. Arguments are normalized so `min <= max`. |
| `SmearorRotation` | Rotation enum from `smearor-wrot-rotation`: `Deg0`, `Deg90`, `Deg180`, `Deg270`, `Deg(f32)`. Re-exported by this crate. In `CardGeometry` it serializes as a plain degree value (`rotation = 180.0`). |
| `MinVisible` | Newtype over `f64` pixels — the minimum overlap a card must keep inside the board bounds. Default `40.0`. |
| `Rect` | GTK-free axis-aligned rectangle (`x`, `y`, `width`, `height`) used for footprints and bounds in the pure math functions. |

### Placement

| Type | Description |
|------|-------------|
| `CardAnchor` | Declarative initial placement: `TopLeft` … `BottomRight` (nine anchors) plus `Explicit(CardPosition)`. Anchors are resolved once the card's natural size and the board bounds are known. |
| `DragHandle` | Which part of a card is the drag affordance: `Header` (default — only the title strip drags, content stays interactive), `WholeCard` (display-only cards), `Handle` (dedicated grip icon in the header). |
| `CardPlacement` | `{ id: CardId, geometry: CardGeometry, folded: bool, seat: Option<Seat>, sticky: bool }` — one entry of a layout, serialized flat; the non-geometry fields default via `serde(default)` for older files. |
| `BoardLayout` | `{ version: u8, workspace, view, seat_anchor, cards: Vec<CardPlacement> }` — the serializable snapshot produced by `layout_snapshot()` and consumed by `restore_layout()`. `version` is reserved for future migrations (currently `1`); the workspace fields default via `serde(default)` so older files parse as `FollowWidget` plus the default view — exactly the pre-workspace behavior. |
| `BoardError` | `UnknownCard(CardId)` \| `DuplicateCard(CardId)` \| `Layout(..)` \| `NoNextSeat` — the crate's `thiserror` error type. |
| `BoundsMode` | `Loose` (default) \| `Contained` — how strictly a card's footprint is kept inside the bounds. |
| `SeatAnchor` | `View` (default) \| `Workspace` — where pass-to-seat operations anchor table edges. |
| `TurnDirection` | `Clockwise` (default) \| `CounterClockwise` — the direction a card is passed around the table. |
| `GotoTransition` | `Snap` \| `Glide` (default) \| `Transit` — how `goto_card` moves the camera. |
| `GotoRotation` | `Keep` \| `Upright` — the destination rotation of a `goto_card` flight. |
| `CardPalette` / `CardColor` | Semantic color set per card (`Background`, `Foreground`, `Header`), turned into a scoped CSS provider. |
| `CardShape` | Card silhouette: `Rectangle`, `Circle`, `Semicircle`, `Hexagon`, `Octagon`, `Annulus` — drives the clip path, `contains` and the band-based chrome layout. |
| `CardContent` | Content allocation on shaped cards: `Inscribed`, `Fill`, `Clipped`, `Cover`. |
| `ChromeElement` / `ChromePlacement` / `ChromeSlot` | Declarative chrome: an element (`Title`, `Minimize`, `Close`, rotate/pass steps, grips, `DragHandle`) plus where it mounts (`Inline*`, `ArcTop*`, `Bar*`, `Satellite`). |
| `ChromeStyle` | Chrome renderer: `Builtin` or `Adwaita`. |
| `CardSatellites` / `SatelliteConfig` / `SatelliteDistribution` | Floating chrome buttons orbiting the card — `Radial` or `Anchored` at `QuarterTurn` angles, with `SatelliteRotationMode`. |

### Configuration (widget layer)

`CardConfig` technically lives in the widget layer but uses only domain
types: `title`, `position: CardAnchor`, `draggable`, `rotatable`, `scalable`,
`foldable`, `closable`, `headerless`, `drag_handle`,
`shape: CardShape`, `content: CardContent`, `hole_ratio`,
`arc_span`/`arc_bottom_span`, `chrome`, `chrome_style: ChromeStyle`,
`satellites`, `scale_limits: ScaleBounds`, `min_visible: MinVisible`,
`bounds_mode: BoundsMode`, `palette`, `z_index`.

## Pure Geometry Functions

All board math is implemented as pure functions over the domain types — no
GTK, no display server, directly unit-testable.

### `rotated_bounds(width, height, degrees) -> (f64, f64)`

Axis-aligned bounding box of a rectangle rotated around its center:

$$\text{bbox}_w = W |\cos\theta| + H |\sin\theta|, \qquad \text{bbox}_h = W |\sin\theta| + H |\cos\theta|$$

This is exactly the size `gtk4::Fixed` allocates for a card stack, so the
board can predict a card's displayed size before it is realized.

### `rotate_vector(x, y, degrees) -> (f64, f64)`

Rotates a vector — used as the fallback for mapping drag offsets from card
space into board space.

### `rotated_region_scanlines(width, height, degrees) -> Vec<(i32, i32, i32)>`

Rasterizes a rotated rectangle into horizontal scanline strips
`(y, x_start, x_end)` relative to its center. The widget layer unions these
into the `cairo::Region` returned by `card.footprint()` — the same
scanline-intersection algorithm `smearor-wrot-rotation` uses for its input
region.

### `resolve_anchor(anchor, bounds, footprint_w, footprint_h) -> CardPosition`

Resolves a `CardAnchor` into a center position. Edge anchors keep the card
`ANCHOR_MARGIN` (24 px) inside the bounds; `Center` centers the card;
`Explicit` passes its position through unchanged.

### `clamp_offset(footprint, bounds, min_visible) -> (f64, f64)`

The boundary-clamping function. Returns the minimal translation so that at
least `min_visible` pixels of `footprint` overlap `bounds` on each axis.
Applying the offset to a card's position pushes it back inside. The required
overlap is capped at half the bounds size so it stays satisfiable on tiny
boards.

## Serialization Format

`BoardLayout` serializes flat per card — designed for `gui-cards.toml`:

```toml
version = 1
workspace = { fixed = [2000.0, 1600.0] }
seat_anchor = "workspace"

[[cards]]
id = "controls"
position = [480.0, 660.0]
rotation = 180.0
scale = 1.0
folded = false
sticky = false
seat = { side = "bottom" }
```

- `position` is the card **center** in workspace coordinates, as an `[x, y]` pair.
- `rotation` is plain degrees (independent of `SmearorRotation`'s own serde
  representation).
- `scale` is the factor (`1.0` = natural size).
- `workspace`, `view` and `seat_anchor` carry `serde(default)` — files from
  before the workspace read the defaults.

Where the file lives and which format is used (TOML, JSON, …) is entirely up
to the consumer — the library only produces and consumes the `BoardLayout`
value. See [Usage](usage.md#persistence) for restore semantics.

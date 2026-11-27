# Usage

## Setup

```toml
[dependencies]
floating-board = "0.1"
```

Serde support (layout snapshots) is enabled by default. Disable it for a
leaner dependency tree — all widget functionality still works:

```toml
[dependencies]
floating-board = { version = "0.1", default-features = false }
```

## Creating a Board

`FloatingBoard::new` takes the widget that forms the bottom layer. Everything
the board renders floats above it; gestures on empty areas fall through.

```rust,ignore
use floating_board::FloatingBoard;

let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
// ... fill content ...
let board = FloatingBoard::new(&content);

board.set_content(&other_widget); // swap the bottom layer later
let overlay = board.overlay();    // internal GtkOverlay (edge cases)
```

## Adding Cards

Every card wraps an arbitrary consumer widget. `add_card` generates a
`card-N` id; `add_card_with_id` assigns a stable semantic key — preferred
whenever you persist layouts, because ids are the keys in `BoardLayout`.

```rust,ignore
use floating_board::CardAnchor;
use floating_board::CardConfig;
use floating_board::DragHandle;
use floating_board::ScaleBounds;

let controls_id = board.add_card_with_id(
    "controls",
    &controls_widget,
    CardConfig {
        title: Some("Controls".into()),
        position: CardAnchor::BottomCenter,
        draggable: true,
        rotatable: true,
        scalable: true,
        foldable: true,
        closable: true,
        drag_handle: DragHandle::Header,
        scale_limits: ScaleBounds::new(0.5, 2.0),
        ..Default::default()
    },
)?;
```

### `CardConfig` Reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `title` | `Option<String>` | `None` | Header text; a localized fallback is used when the header is shown without a title |
| `position` | `CardAnchor` | `Center` | Initial anchor (resolved at first allocation) or `Explicit(CardPosition)` |
| `draggable` | `bool` | `true` | Enable the drag gesture |
| `rotatable` | `bool` | `true` | Enable the two-finger rotate gesture |
| `scalable` | `bool` | `true` | Enable the pinch gesture |
| `resizable` | `bool` | `false` | Show a bottom-right resize grip that scales around the center |
| `rotate_grip` | `bool` | `false` | Show a bottom-left rotate grip that quarter-turns in the swipe direction |
| `foldable` | `bool` | `false` | Show a fold button; folds the card to its header |
| `closable` | `bool` | `false` | Show a close button; removes the card (`card-removed`) |
| `headerless` | `bool` | `false` | Collapse the header to a slim grip |
| `drag_handle` | `DragHandle` | `Header` | `Header`, `WholeCard` or `Handle` (grip icon) |
| `shape` | `CardShape` | `Rectangle` | `Rectangle`, `Circle`, `Semicircle`, `Hexagon`, `Octagon`, `Annulus` — see [Shapes](cards/shapes.md) |
| `content` | `CardContent` | `Inscribed` | `Inscribed`, `Fill`, `Clipped`, `Cover` — how content fills the shape |
| `hole_ratio` | `Option<f64>` | `0.5` | `Annulus` inner radius as a fraction of the outer radius |
| `arc_span` / `arc_bottom_span` | `Option<Degrees>` | `90°` | Width of the top/bottom arc bands on curved cards |
| `chrome` | `Option<Vec<ChromeSlot>>` | legacy | Explicit `ChromeElement` + `ChromePlacement` list; `Some(vec![])` = chrome-free |
| `chrome_style` | `ChromeStyle` | `Builtin` | `Builtin` or `Adwaita` (feature `adwaita`) |
| `satellites` | `Option<CardSatellites>` | `None` | Floating chrome buttons around the card (`Radial`/`Anchored`) |
| `scale_limits` | `ScaleBounds` | `0.5 – 2.0` | Inclusive pinch zoom limits |
| `min_visible` | `MinVisible` | `40.0` | Pixels that must stay inside the bounds (`Loose` mode) |
| `bounds_mode` | `BoundsMode` | `Loose` | `Loose` clamps on drag end; `Contained` keeps the whole footprint inside, live |
| `anchor_margin` | `Pixels` | `24.0` | Margin edge/corner anchors keep to the board edge |
| `palette` | `Option<CardPalette>` | `None` | Per-card color scheme (`CardColor::Background`, `Foreground`, `Header`) |
| `content_padding` | `Pixels` | `10.0` | Space between the card border and the content widget |
| `max_width` / `max_height` | `Option<i32>` | `None` | Hard cap on the measured size — keeps the card stable when content changes size |
| `animation_speed_ms` | `u64` | `500` | Duration of rotation and layout animations |
| `seat` | `Option<Seat>` | `None` | Tabletop seat the card is assigned to — independent of its visual rotation |
| `sticky` | `bool` | `false` | Pin the card to the screen: it keeps position, rotation and zoom while the view moves |
| `folded_content` | `Option<Widget>` | `None` | Custom widget shown while folded instead of the header strip (round button, compact view) |
| `z_index` | `Option<i32>` | `None` | Initial stacking order; `None` appends on top |

`CardConfig::default()` yields a fully interactive card anchored at the board
center.

## Working with Cards

```rust,ignore
let card = board.card(&controls_id)?;   // Result<Card, BoardError>
let all = board.cards();                // Vec<Card>

// Geometry — the board is the single state owner.
let geometry = card.geometry();
card.set_position(CardPosition::new(480.0, 300.0));
card.set_scale(ScaleFactor::new(1.5));
card.set_rotation(SmearorRotation::Deg180);                // immediate
card.set_rotation_with_animation(SmearorRotation::Deg180); // zoom-snap transition
card.set_geometry(&CardGeometry { position, rotation, scale });

// Ordering and chrome.
card.raise();
card.set_folded(true);   // or card.fold() to toggle
let folded = card.folded();
card.close();            // runs the configured close action (remove or hide)
```

`CardConfig::folded_content` replaces the whole card while folded,
header included: folding hides the chrome and shows that widget
instead (`.floating-card-collapsed` makes the card background
transparent). Wire the folded widget to `set_folded(false)` or
`fold()` - the card's own fold button hides with the header. A round
icon button or a compact mini toolbar both work:

```rust,ignore
let collapsed = Button::from_icon_name("open-menu-symbolic");
board.add_card(&body, CardConfig {
    foldable: true,
    folded_content: Some(collapsed.clone().upcast()),
    ..Default::default()
});
let card = board.card(&id)?;
collapsed.connect_clicked(move |_| card.set_folded(false));


// The rotated footprint in board coordinates — union of all footprints is
// the dynamic safe-area of the board.
let region: gtk4::cairo::Region = card.footprint();

board.remove_card(&controls_id)?;
```

## Layouts and Seats

`smearor-commons-layout` provides the generic machinery — the board only
adapts card state to and from it.

### Applying a layout strategy

```rust,ignore
use smearor_commons_layout::FlowLayout;

// Line up all visible cards without overlap, gliding them animatedly:
board.apply_layout(&FlowLayout::default(), None, true)?;

// Or only a selection of cards:
board.apply_layout(&FlowLayout::default(), Some(&[id_a, id_b]), false)?;

// Any `LayoutStrategy<Pixels>` works - `SeatLayout`, or your own:
board.apply_layout(&SeatLayout::default(), None, true)?;
```

`apply_layout` maps each card to a `LayoutItem` (displayed footprint,
live rotation, `seat`), calls `arrange` against the board bounds and
applies every `Placement` through the regular geometry path — clamping,
scale limits and `geometry-changed` behave like manual manipulation.
`Placement::size` is the target footprint; cards keep their configured
scale. A `BoardError::Layout(DoesNotFit)` leaves the board untouched.

### Seats

A `Seat` is a `TableSide` plus an optional `Fraction` offset along the
edge. It models *which user a card belongs to* — deliberately decoupled
from the card's rotation, because the board may be rotated and users
may move:

```rust,ignore
let config = CardConfig {
    seat: Some(Seat::new(TableSide::Bottom)),            // assigned, strategy distributes along the edge
    // seat: Some(Seat::at(TableSide::Top, Fraction::new(0.25))), // pinned position
    // seat: Some(Seat::center(TableSide::Left)),                // centered on an edge
    ..Default::default()
};

let seat = card.seat();            // Option<Seat>
card.set_seat(Some(Seat::center(TableSide::Right)));  // emits card-seat-changed
board.set_card_seat(&id, None)?;   // clear via the board
```

Seats are part of `layout_snapshot()`/`restore_layout()` via
`CardPlacement::seat`.

### Passing a card to the next user

```rust,ignore
// Step one table side (Bottom -> Left -> Top -> Right), gliding to the
// edge and rotating into that side's reading orientation:
board.rotate_to_next_user(&id, TurnDirection::Clockwise, true)?;

// Or hop between *occupied* seats of a roster — empty sides are
// skipped, several seats on one side are visited in order:
let roster = [Seat::new(TableSide::Bottom), Seat::center(TableSide::Top)];
board.rotate_to_next_occupied_seat(&id, &roster, TurnDirection::Clockwise, true)?;
```

`move_to_edge` (usually `true`) slides the card in front of the target
edge at `anchor_margin`; `false` rotates in place.
`rotate_to_next_occupied_seat` assigns the reached roster seat to the
card (`card-seat-changed`) — being passed to a user makes it belong to
their seat, which is what lets repeated passes keep hopping the roster.
`rotate_to_next_user` only advances an explicitly assigned seat;
unassigned cards derive their starting side from their rotation each
time. A passed card slips into the free spot nearest its seat: it
fills gaps left by removed cards along the edge, sits beside occupied
cards without overlap, and may take the lane behind the edge row when
the queue there reaches farther from the seat than a second row would
— the edge lane is preferred.
`BoardError::NoNextSeat` is returned when the roster holds no seat
other than the card's current one.

## Workspace and View

Card geometry — position, rotation and scale — is stored in *workspace
coordinates* on a virtual surface. A `View` camera decides which part of
the workspace maps onto the widget, so panning, zooming or rotating the
table never touches card state. The defaults make the widget a window
exactly covering the workspace — pre-workspace behavior.

### Workspace extent

```rust,ignore
use smearor_commons_layout::WorkspaceSize;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;

board.set_workspace_size(WorkspaceSize::FollowWidget);                     // default: mirrors the allocation
board.set_workspace_size(WorkspaceSize::Fixed(Vec2::<Pixels>::new(2000.0, 1600.0))); // a table bigger than the widget
board.set_workspace_size(WorkspaceSize::Infinite);                          // unbounded
```

- `FollowWidget` (default): the workspace extent tracks the widget
  allocation — identical behavior to pre-workspace versions.
- `Fixed(size)`: a fixed workspace extent in pixels, origin at (0,0).
- `Infinite`: no extent; clamping falls back to the shown view region
  (so cards cannot be dragged off-screen and lost).

### The camera

```rust,ignore
let view = board.view();                 // effective View { region, rotation }
board.set_view(view);                    // pin an explicit view
board.reset_view();                      // back to the default covering the extent

board.pan_view(Vec2::<Pixels>::new(100.0, 0.0));                // shift by workspace pixels
board.zoom_view(anchor, ScaleFactor::new(1.25));                // zoom in around a workspace point
board.rotate_view(pivot, RotationRad::from_degrees(90.0));      // turn the table around a point
```

The default view covers the workspace extent and follows the widget.
`set_view`, `pan_view`, `zoom_view` and `rotate_view` pin an explicit
view — resizes no longer change zoom or center until `reset_view`
releases the pin. `View` itself (`smearor_commons_layout`) offers
`to_widget`/`to_workspace` for coordinate mapping and `shown_region`
for the aspect-expanded region actually on screen.

### User control

Both controls are opt-in (default off) because they share the touch
surface with card gestures:

- `board.set_view_gestures(true)` — drag on empty space pans, a pinch
  zooms around the gesture center and a two-finger twist rotates the
  table. Card gestures keep precedence: they claim sequences first,
  and touches starting on a card are denied up front.
- `board.set_view_buttons(true)` — overlaid corner controls (zoom in,
  zoom out, reset) styled via `.floating-board-view-controls`.

### `goto_card`

```rust,ignore
use floating_board::GotoRotation;
use floating_board::GotoTransition;

// Fly the camera onto a card, turning it readable for the viewer:
board.goto_card(&id, GotoTransition::Glide, GotoRotation::Upright)?;

// Prezi-style: pull back to an overview of both regions first,
// then glide down onto the card:
board.goto_card(&id, GotoTransition::Transit, GotoRotation::Upright)?;

// Or just bring it on screen without turning the table:
board.goto_card(&id, GotoTransition::Snap, GotoRotation::Keep)?;
```

The target view centers the card's workspace footprint with a padding
margin. `GotoTransition` picks the flight (`Snap`, `Glide`, `Transit`),
`GotoRotation` the destination rotation (`Upright` rotates so the card
reads upright, `Keep` pans/zooms only). Flights run on the frame clock
over the card's `animation_speed_ms`; a new `goto_card` supersedes a
running one and any programmatic or gesture view change cancels it.
The destination pins like `set_view` — `reset_view` releases it.
A `sticky` target is pinned to the glass itself, so the camera moves
to the workspace region behind it and `rotation` is ignored.

### Sticky cards

`CardConfig::sticky` (or `card.set_sticky(true)`) pins a card to the
screen instead of the workspace. A sticky card keeps its on-screen
position, rotation and zoom no matter how the view pans, zooms or
rotates — like chrome glued to the glass rather than a sheet on the
table. It still clamps against the widget rect, and its gestures work
in screen coordinates.

```rust,ignore
board.add_card("sticky-hud", &content, CardConfig {
    position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(900.0, 60.0))),
    sticky: true,
    ..Default::default()
});

// Toggling never moves the card visually: the stored geometry
// converts between workspace and widget coordinates at the seam.
card.set_sticky(false); // release back onto the table
```

While pinned, the card's stored position is in widget pixels (also in
`Card::geometry()` and layout snapshots); all other cards use
workspace coordinates. Sticky cards are skipped by `apply_layout`,
count as obstacles for seat passes only under `SeatAnchor::View`, and
passing one to a seat releases the pin first — a card handed to
another user always lands on the table.

### Seats and the camera

`board.set_seat_anchor` chooses where pass-to-seat operations anchor
their table edges:

- `SeatAnchor::View` (default): seats sit at the *screen* edge — a
  passed card lands where the user actually sits, however the camera
  is panned, zoomed or rotated.
- `SeatAnchor::Workspace`: seats are fixed positions on the virtual
  table — they stay where the workspace puts them.

### Persistence and clamping

`BoardLayout` persists `workspace` and `view` (via `serde(default)` —
snapshots from before the workspace read the defaults). Cards stored
out of bounds are clamped on restore as usual. `Infinite` workspaces
clamp card *operations* against the shown view region but never
re-clamp on view changes, so panning cannot drag cards along.

## Persistence

Snapshots are value objects; file location and format belong to the consumer.

```rust,ignore
// Capture
let layout: BoardLayout = board.layout_snapshot();
let toml = toml::to_string_pretty(&layout)?;

// Restore (e.g. on startup or --reset-gui-layout)
let layout: BoardLayout = toml::from_str(&file_contents)?;
board.restore_layout(&layout);
```

**Restore semantics:**

- Unknown `CardId`s are skipped with a `warn!` log — profiles may reference
  cards that no longer exist; this is not an error.
- Geometry is clamped to the current bounds, so nothing restores off-screen.
- Cards missing from the layout keep their current geometry.

Example `gui-cards.toml`:

```toml
version = 1

[[cards]]
id = "controls"
position = [480.0, 660.0]
rotation = 180.0
scale = 1.0
folded = false
sticky = false
seat = { side = "bottom" }
```

## Signals

Connect with `connect_closure` or the generated `connect_*` helpers:

```rust,ignore
// Aggregate channel — debounce your persistence here.
board.connect_closure("geometry-changed", false,
    gtk4::glib::closure_local!(move |board: FloatingBoard| {
        save(board.layout_snapshot());
    }),
);

// Per-card lifecycle.
card.connect_closure("card-moved", false,
    gtk4::glib::closure_local!(|card: floating_board::Card| {
        println!("{} moved", card.id());
    }),
);
```

| Signal | Source | Fires on |
|--------|--------|----------|
| `card-moved` | `Card` | drag end |
| `card-rotated` | `Card` | any rotation change (gesture or programmatic) |
| `card-scaled` | `Card` | transform end or `set_scale` |
| `card-raised` | `Card` | raise to top |
| `card-folded` | `Card` | fold state toggles |
| `card-seat-changed` | `Card` | seat assignment changes (`set_seat`, pass to user) |
| `card-added` / `card-removed` | `FloatingBoard` | card lifecycle (carries the `Card`) |
| `card-visibility-changed` | `FloatingBoard` | `set_card_visible` toggles or `CardCloseAction::Hide` hides the card (carries the `Card`) |
| `geometry-changed` | `FloatingBoard` | aggregate: any geometry mutation |

## Styling

The crate ships a default stylesheet registered with
`STYLE_PROVIDER_PRIORITY_FALLBACK` — every rule is overridable by your theme:

| Class | Element |
|-------|---------|
| `.floating-board` | the board itself |
| `.floating-card-layer` | the transparent card layer |
| `.floating-card` | every card |
| `.floating-card-header` | the header strip |
| `.floating-card-content` | the content area |
| `.floating-card-handle` | the grip icon (`DragHandle::Handle`) |
| `.floating-card-folded` | added while folded |
| `.floating-card-collapsed` | added while folded with `folded_content` (chrome stripped) |
| `.floating-card-folded-content` | wrapper around `folded_content` while folded |
| `.floating-card-headerless` | added for `headerless` cards |

The few crate-owned strings (untitled fallback, fold/close tooltips) are
localized via Fluent (`i18n/{en,de}/main.ftl`, selected by `LANGUAGE`/`LANG`).
All card content is consumer-owned and unaffected.

## Demo

```bash
cargo run --example demo
```

Shows four cards on a drawing area inside the board's content area —
the toolbar and status line sit outside, so cards can never cover them.
The controls card is foldable/closable (`Header` drag), the info card is
rotated 90° (`Handle` drag, `min_visible = 60`), the display card is
headerless (`WholeCard` drag) and the parameters card folds.

Four demo users sit at seats around the table edge — colored markers on
the background, a matching seat badge on each card. The toolbar adds
cards at runtime, exercises `layout_snapshot`/`restore_layout` and
drives the layouting API: "Arrange" flows the visible cards into a
non-overlapping row, "Pass card →" sends the controls card to the next
user — hopping the occupied seats when the "Seats" switch is on
(`rotate_to_next_occupied_seat`), stepping table sides when off. In
seat mode the markers themselves are draggable along the table edge.
"View gestures"/"View buttons"/"Seats @ view" toggle the camera and
seat anchoring live; "Goto card" and each dynamic card's "Goto" fly
the camera onto a card — "Prezi goto" selects the transit flight,
"Goto upright" turns the card readable.
A status line shows signals live.

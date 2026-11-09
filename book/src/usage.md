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
| `foldable` | `bool` | `false` | Show a fold button; folds the card to its header |
| `closable` | `bool` | `false` | Show a close button; removes the card (`card-removed`) |
| `headerless` | `bool` | `false` | Collapse the header to a slim grip |
| `drag_handle` | `DragHandle` | `Header` | `Header`, `WholeCard` or `Handle` (grip icon) |
| `scale_limits` | `ScaleBounds` | `0.5 – 2.0` | Inclusive pinch zoom limits |
| `min_visible` | `MinVisible` | `40.0` | Pixels that must stay inside the bounds (boundary clamping) |
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

// The rotated footprint in board coordinates — union of all footprints is
// the dynamic safe-area of the board.
let region: gtk4::cairo::Region = card.footprint();

board.remove_card(&controls_id)?;
```

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
| `card-scaled` | `Card` | pinch end or `set_scale` |
| `card-raised` | `Card` | raise to top |
| `card-folded` | `Card` | fold state toggles |
| `card-added` / `card-removed` | `FloatingBoard` | card lifecycle (carries the `Card`) |
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
| `.floating-card-headerless` | added for `headerless` cards |

The few crate-owned strings (untitled fallback, fold/close tooltips) are
localized via Fluent (`i18n/{en,de}/main.ftl`, selected by `LANGUAGE`/`LANG`).
All card content is consumer-owned and unaffected.

## Demo

```bash
cargo run --example demo
```

Shows a drawing area as the bottom layer plus four cards: a foldable and
closable controls card (`Header` drag), a 90°-rotated info card (`Handle`
drag, `min_visible = 60`), a headerless display card (`WholeCard` drag) and a
foldable parameters card. The toolbar adds cards at runtime and exercises
`layout_snapshot`/`restore_layout` (in-memory, JSON is logged); a status line
shows `geometry-changed`, `card-added` and `card-removed` live.

# Chrome

`CardConfig` decides how much window-manager chrome a card gets —
from a fully titled window with fold and close buttons down to a
chrome-less floating content widget.

## Drag handles

`DragHandle` selects which part of the card is the move affordance:

| Variant | Effect |
|---|---|
| `Header` (default) | only the title strip drags — content stays fully interactive |
| `WholeCard` | the entire surface drags — for display-only cards whose content needs no input |
| `Handle` | a dedicated grip icon in the header is the only drag surface — title clicks stay free for content |

`draggable: false` disables the drag entirely; the card can still be
rotated, scaled and laid out.

## Folds and closes

```rust,ignore
CardConfig {
    foldable: true,                        // fold button in the header
    closable: true,                        // close button in the header
    close_action: CardCloseAction::Hide,   // or Remove (default)
    folded_content: Some(mini_toolbar),    // replaces the whole card while folded
    ..Default::default()
}
```

`CardCloseAction` decides what the close button means:

- **`Remove`** (default): destroys the card — `card-removed` fires and
  the `CardId` is gone from `board.cards()`.
- **`Hide`**: collapses the card in place — it keeps its geometry,
  stays listed and comes back with
  `board.set_card_visible(&id, true)`. Suits menu-driven boards that
  open and close the same panels repeatedly.

`board.close_card(&id)` runs the configured action programmatically;
`is_card_visible` reports the state.

## Chrome slots and placements

The `foldable`/`closable` convenience flags are sugar over the real
model: `chrome` takes an explicit `Vec<ChromeSlot>` — a `ChromeElement`
plus a `ChromePlacement` that says *where* it mounts.

```rust,ignore
chrome: Some(vec![
    ChromeSlot::inline(ChromeElement::DragHandle),
    ChromeSlot::inline(ChromeElement::Title),
    ChromeSlot::inline_end(ChromeElement::Minimize),
    ChromeSlot::bar(ChromeElement::RotateCounterClockwise),
    ChromeSlot::bar(ChromeElement::RotateClockwise),
    ChromeSlot::satellite(ChromeElement::Close),
]),
```

Elements: `DragHandle`, `Title`, `Minimize`, `Close`,
`RotateClockwise`/`RotateCounterClockwise` (snap-step buttons),
`NextSeat`/`PreviousSeat` (pass around the table), `ResizeGrip` and
`RotateGrip` (the bottom-corner drags). Placements: `Inline*`
(header strip on `Rectangle`, top arc band on curved shapes),
`ArcTop*` (explicit top-arc zones), `Bar*` (bottom band — arc on
`Circle`/`Annulus`, straight strip on `Semicircle`/`Hexagon`/`Octagon`)
and `Satellite`. See [Shapes](shapes.md#chrome-on-shaped-cards) for the
band layout and mirroring rules.

Two subtleties:

- `chrome: None` does **not** mean "no chrome" — it falls back to the
  legacy slot list derived from the flags (always `Title`, plus
  `Minimize`/`Close` when `foldable`/`closable`). A truly chrome-free
  card is `chrome: Some(vec![])`; pair it with `drag_handle: WholeCard`.
- `chrome_style` selects the renderer: `ChromeStyle::Builtin` (stock
  GTK4/CSS) or `ChromeStyle::Adwaita` (`adw::HeaderBar`, behind the
  `adwaita` feature).

## Satellites

`satellites: Option<CardSatellites>` mounts small floating buttons
around the card — the same `ChromeElement`s, freed from the strip,
plus consumer widgets of any kind. Layouts (`Radial` orbit, `Sidebar`
strip), rotation and collision modes, and the open/close menu pattern
have their own page: [Satellites](satellites.md).

## Per-card colors

`palette` accepts a `CardPalette` — a `Palette<CardColor, f64>` from
`smearor-commons-color` — and turns it into a scoped CSS provider at
build time:

```rust,ignore
use floating_board::CardColor;
use floating_board::CardPalette;
use smearor_commons_color::Color;

let mut palette = CardPalette::new();
palette.insert(CardColor::Background, Color::<f64>::new(0.1, 0.12, 0.18));
palette.insert(CardColor::Foreground, Color::<f64>::new(0.9, 0.9, 0.9));
palette.insert(CardColor::Header,      Color::<f64>::new(0.16, 0.2, 0.3));
```

Slots: `Background` (card surface), `Foreground` (title, buttons, grip
strokes), `Header` (header strip). Unfilled slots keep the themed
colors — a palette may fill any subset. Per-seat card colors are the
idiomatic use: each player's cards carry their marker color.

## Layout-related flags

| Field | Effect |
|---|---|
| `headerless` | collapses the header to a slim grip |
| `shape` | `CardShape` — `Rectangle` (default), `Circle`, `Semicircle`, `Hexagon`, `Octagon`, `Annulus`; see [Shapes](shapes.md) |
| `content` | `CardContent` allocation mode — `Inscribed` (default), `Fill`, `Clipped`, `Cover` |
| `hole_ratio` | `Annulus` only: inner radius as a fraction of the outer (default 0.5) |
| `arc_span` / `arc_bottom_span` | angular width of the top/bottom arc bands on curved cards (default a quarter turn) |
| `content_padding` | inset between card border and content (default 10 px; `Inscribed` only — immersive modes allocate to the silhouette) |
| `max_width` / `max_height` | hard cap on measured card size |
| `z_index` | initial stacking order; `None` appends on top |
| `animation_speed_ms` | duration of rotation, glide and camera animations on this card (default 500 ms) |

`CardConfig::default()` yields a fully interactive card: draggable,
rotatable, scalable, header shown, `Center` anchored. Everything else
is opt-in.

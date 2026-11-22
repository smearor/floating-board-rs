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
| `content_padding` | inset between card border and content (default 10 px) |
| `max_width` / `max_height` | hard cap on measured card size |
| `z_index` | initial stacking order; `None` appends on top |
| `animation_speed_ms` | duration of rotation, glide and camera animations on this card (default 500 ms) |

`CardConfig::default()` yields a fully interactive card: draggable,
rotatable, scalable, header shown, `Center` anchored. Everything else
is opt-in.

# Sticky Cards

`CardConfig::sticky` pins a card to the **screen** instead of the
workspace — chrome glued to the glass rather than a sheet on the
table. A sticky card keeps its on-screen position, rotation and zoom
no matter how the view pans, zooms or rotates; HUD-style toolbars
are the idiomatic use.

```rust,ignore
board.add_card(&hud, CardConfig {
    position: CardAnchor::Explicit(CardPosition::new(Vec2::<Pixels>::new(900.0, 60.0))),
    sticky: true,
    ..Default::default()
});

card.set_sticky(true);   // pin to the glass
card.set_sticky(false);  // release back onto the table
```

## The coordinate seam

Toggling never moves the card visually: at the seam, its stored
`CardPosition` converts between workspace and widget coordinates
through the current `View` mapping. Two consequences to keep in mind:

- **While pinned**, `Card::geometry()` (and layout snapshots) report
  *widget* coordinates; for every other card they report workspace
  coordinates.
- Gestures still work — in screen coordinates — and the card still
  clamps, against the widget rect rather than the workspace.

## Layout interactions

Sticky cards are deliberately excluded from table semantics:

- `apply_layout` skips them — strategies arrange table content, not
  chrome.
- They count as obstacles for seat passes only under
  `SeatAnchor::View` (where seats live on screen anyway).
- Passing a sticky card to a seat **releases the pin first** — a card
  handed to another user always lands on the table.
- A `goto_card` onto a sticky card flies to the workspace region
  behind it; `GotoRotation` is ignored.

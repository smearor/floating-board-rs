# Introduction

Welcome to the documentation for `floating-board`.

`floating-board` is a GTK4 library providing a single composite widget —
`FloatingBoard` — that renders freely movable, rotatable and scalable cards on
top of an application's content. Think of it as an in-app window manager for
touch surfaces.

## Why floating-board?

The widget is designed for **table-top multitouch surfaces**: screens built
into a table where several users stand on different sides and interact with
the same surface. On such a table there is no "up" and no "down" — content
must be orientable for every user. `floating-board` makes this practical:

1. **Cards move, rotate and scale independently.** Each card is a regular GTK4
   widget that users can drag across the surface, rotate towards themselves
   with a two-finger gesture, and resize with a pinch gesture.
2. **The board is a transparent overlay.** Cards float above the application
   content. Touches that do not land on a card fall through to the widgets
   underneath — no input grab, no modal layer.
3. **Multiple users at once.** GTK touch sequences are independent per card:
   one user can drag a card on the left while another rotates a card on the
   right.
4. **Cards cannot get lost.** Boundary clamping keeps at least a configurable
   slice of every card inside the visible area, even while it is being pushed,
   turned or scaled.

## Use Cases

- **Games**: place the game controls, score displays or hand-of-cards on the
  table surface and let each player orient them as needed.
- **Presentations**: put information on cards that viewers on different sides
  of the table can pull closer, rotate and enlarge.
- **Table-top UIs**: build interfaces that are usable from every side of the
  table at the same time.

## Feature Overview

| Feature | Description |
|---------|-------------|
| Move | One-finger drag via header, handle or the whole card |
| Rotate | Two-finger rotation (snaps to 90° steps) via `smearor-wrot-rotation` |
| Scale | Two-finger pinch zoom with per-card scale limits |
| Boundary clamping | `min_visible` pixels of every card stay inside the board |
| z-order | Touching a card raises it; initial order via `z_index` |
| Persistence | `BoardLayout` snapshot/restore (TOML/JSON), serde behind a feature flag |
| Card chrome | Optional title, fold-to-header, close button, headerless mode |
| Multi-touch | Independent gesture sequences per card |

## Getting Started

Add the dependency and create a board over your existing content:

```toml
[dependencies]
floating-board = "0.1"
```

```rust,ignore
let board = FloatingBoard::new(&content_widget);
let controls = board.add_card(
    &controls_widget,
    CardConfig { title: Some("Controls".into()), position: CardAnchor::BottomCenter, ..Default::default() },
);
```

Continue to [Usage](usage.md) for the full walkthrough, or read about the
[Domain Model](domain_model.md) and [Architecture](architecture.md) first.

## Running the Demo

```bash
cargo run --example demo
```

The demo shows a drawing area as background content plus four cards with
different interaction profiles (header drag, handle drag, whole-card drag,
foldable/closable chrome, 90° rotation), a toolbar for adding cards and
snapshot/restore, and a live status line fed by the board's signals.

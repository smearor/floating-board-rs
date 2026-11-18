# floating-board-rs

[![Version](https://img.shields.io/badge/version-0.1.0-f5b700.svg)](https://github.com/smearor/floating-board-rs)
[![Rust Edition](https://img.shields.io/badge/rust-2024-00a1e4.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![GTK4](https://img.shields.io/badge/GTK4-v4__20-dc0073.svg)](https://gtk-rs.org/gtk4-rs/stable/latest/docs/gtk4/)
[![License](https://img.shields.io/badge/license-MIT-04e762.svg)](LICENSE)
[![Book](https://img.shields.io/badge/book-mdBook-89fc00.svg)](https://smearor.github.io/floating-board-rs/book/)
[![Docs](https://img.shields.io/badge/docs-rustdoc-04e762.svg)](https://smearor.github.io/floating-board-rs/docs/)
[![crates.io](https://img.shields.io/badge/crates.io-publish-dc0073.svg)](https://crates.io/crates/floating-board)

`floating-board-rs` is a GTK4 widget that renders freely floating cards on a surface. Cards can be moved, rotated and scaled independently,
folded, closed and rearranged, designed for multi-touch tabletop applications where several users interact from different sides of a table.

---

## What is Smearor Smart Desk Table Top?

Smearor Smart Desk is a project for interactive table surfaces: a large horizontal multi-touch display built into a desk. Several people can
stand or sit around the table and use applications at the same time, each from their own side. There is no natural "top" or "bottom" of the
screen, because every viewer faces a different direction.

Software for such a surface has special requirements that regular desktop toolkits do not cover:

- Content must be readable and operable from any side of the table, so users can rotate elements freely.
- Several hands and several users must be able to manipulate different elements simultaneously.
- Elements belong to people sitting at an edge, not to a fixed screen corner, and can be passed from one side to another.
- Interfaces need a sense of physical space: objects stay where they are put and can be organized around the table like cards on a real desk.

The Smearor ecosystem (for example `smearor-commons` for geometry, animation and layout primitives, `smearor-wrot-rotation` for rotated GTK4
widgets, and `tangible-objects` for detecting physical objects placed on the surface) provides the building blocks for these applications.
`floating-board-rs` is the workspace widget that ties them together.

## About Floating Board

`FloatingBoard` is an overlay widget: your application content forms the bottom layer, and cards float on a transparent layer above it.
Gestures that do not hit a card fall through to your content, so the board behaves like an in-app window manager for tabletop UIs.

Features:

- **Free card manipulation**: every card can be moved, rotated and scaled independently through multi-touch gestures (drag, two-finger
  rotate, pinch, and an N-point transform tracker that combines all three). Cards can be manipulated by several users at the same time.
- **Card chrome**: optional title header, fold to header, close button, resize grip, rotate grip, and selectable drag handles (header,
  whole card or a grip icon). Headerless and fully chrome-less cards are supported.
- **Virtual workspace and camera**: card geometry lives in workspace coordinates on a surface that can be `FollowWidget` sized, `Fixed`
  sized or `Infinite`. A `View` camera maps the workspace onto the widget and can be panned, zoomed and rotated, per flag-gated gestures,
  overlaid control buttons or programmatic API.
- **`goto_card` camera flights**: fly the camera onto a card and center it, optionally turning it upright for the viewer. `GotoTransition`
  selects between `Snap`, a direct `Glide`, and a `Transit` flight that pulls back to an overview first, Prezi style.
- **Tabletop seats**: a `Seat` (table side plus optional position along the edge) assigns cards to users independently of card rotation.
  Cards can be passed to the next user or to the next occupied seat, landing in front of that seat in the user's reading orientation and
  fanning out to the nearest free spot. `SeatAnchor` chooses whether seats live at the visible screen edge or on the virtual table.
- **Layout strategies**: `apply_layout` arranges cards through `smearor-commons-layout` strategies (`FlowLayout`, `SeatLayout`, or your own
  `LayoutStrategy` implementation), animated via per-card frame-clock glides.
- **Boundary clamping**: cards keep a configurable minimum of their rotated footprint inside the bounds, live (`Contained`) or on gesture
  end (`Loose`).
- **Serializable state**: `layout_snapshot()` / `restore_layout()` produce a `BoardLayout` (geometry, fold state, seats, workspace and view)
  ready for TOML or JSON persistence. Persistence itself stays with the consumer.
- **Signals and styling**: per-card and board signals (`card-moved`, `card-seat-changed`, `geometry-changed`, and more) for live persistence
  hooks; a default stylesheet registered at fallback priority, per-card color palettes, and Fluent localization (English and German) for
  the few crate-owned strings.
- **gtk-free domain core**: all geometry math, layout serde and state types live in `src/domain`, independently testable and free of GTK.

Built on [`smearor-commons`](https://github.com/smearor/smearor-commons) (geometry, animation, layout) and
[`smearor-wrot-rotation`](https://github.com/smearor/smearor-wrot-rotation) (the `RotationWidget` inside each card).

## Integrating Floating Board into your application

Add the dependency:

```toml
[dependencies]
floating-board = { git = "https://github.com/smearor/floating-board-rs.git" }
```

Create a board around your content widget and add cards. Every card wraps an arbitrary `gtk4::Widget`:

```rust
use floating_board::{CardAnchor, CardConfig, DragHandle, FloatingBoard};

let content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
// ... fill content ...
let board = FloatingBoard::new(&content);

let card_id = board.add_card_with_id(
    "controls",
    &controls_widget,
    CardConfig {
        title: Some("Controls".into()),
        position: CardAnchor::BottomCenter,
        foldable: true,
        closable: true,
        drag_handle: DragHandle::Header,
        ..Default::default()
    },
)?;
```

A few common follow-ups:

```rust
// Enable the camera controls for your users:
board.set_view_gestures(true);
board.set_view_buttons(true);

// Arrange all cards without overlap, animated:
board.apply_layout(&smearor_commons_layout::FlowLayout::default(), None, true)?;

// Fly the camera onto a card:
board.goto_card(&card_id, GotoTransition::Transit, GotoRotation::Upright)?;

// Persist and restore the layout (format and location are yours):
let layout = board.layout_snapshot();
board.restore_layout(&layout);
```

The [usage chapter of the book](https://smearor.github.io/floating-board-rs/book/usage.html) documents the full `CardConfig` reference,
seats and passing, the workspace/view model, signals and styling. Run the bundled demo to see everything in action:

```bash
cargo run --example demo
```

## License

`floating-board-rs` is licensed under the [MIT License](LICENSE), Copyright (c) 2026 Smearor.

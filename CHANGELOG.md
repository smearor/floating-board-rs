# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `FloatingBoard` composite widget: internal `GtkOverlay` stacking a transparent `GtkFixed` card layer over the consumer's content widget; gestures on empty areas fall through to the content.
- `Card` widgets (`ScalingWidget` > `RotationWidget` > `Card` transform stack) that can be moved, rotated and scaled independently, designed for table-top multitouch surfaces.
- Interaction gestures: header/handle/whole-card drag (mapped into board coordinates through the rotate/scale transform stack), two-finger rotation via `smearor-wrot-rotation`, pinch zoom via `GestureZoom`, click/drag-end z-order raise.
- Boundary clamping: cards keep at least `CardConfig::min_visible` pixels of their rotated footprint inside the board bounds (drag end, rotation and scale changes, board resize).
- Strict gtk-free domain model in `domain/`: `CardId`, `CardPosition` (card center), `ScaleFactor`, `ScaleBounds`, `MinVisible`, `CardAnchor` (9 anchors + explicit), `DragHandle`, `CardGeometry`, `CardPlacement`, `BoardLayout`, `BoardError`, plus pure anchor resolution, clamping and rotated-footprint math.
- Serializable layout snapshots: `layout_snapshot()`/`restore_layout()` produce a versioned `BoardLayout` (TOML/JSON via the default `serde` feature); persistence remains consumer-side.
- `CardConfig` per-card interaction flags: `title`, `position`, `draggable`, `rotatable`, `scalable`, `foldable`, `closable`, `headerless`, `drag_handle`, `scale_limits`, `min_visible`, `z_index`.
- Signals: `card-moved`, `card-rotated`, `card-scaled`, `card-raised`, `card-folded` on `Card`; `card-added`, `card-removed`, `geometry-changed` on `FloatingBoard`.
- Foldable and closable cards, localized crate strings via Fluent (`i18n/{en,de}/main.ftl`).
- Default card stylesheet (`.floating-card` and related classes, registered with fallback priority).
- `examples/demo.rs`: runnable demo (`cargo run --example demo`) with background content, four cards, dynamic card creation, snapshot/restore and live signal status.

### Fixed

- Dragging a card followed the pointer at roughly half speed and jittered: the drag delta was mapped through `compute_point` under the live (drifting) transform. The delta is now derived from board-space pointer samples taken from event surface coordinates.
- Widgets outside the cards received no pointer input: `smearor-wrot-rotation`'s `RotationWidget` narrows the surface input region to each card's footprint on every allocation (designed for layer-shell surfaces). The board now restores the full input region at the end of each `size_allocate` pass.

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
- `CardConfig` per-card interaction flags: `title`, `position`, `draggable`, `rotatable`, `scalable`, `foldable`, `closable`, `headerless`, `drag_handle`, `anchor_margin`, `scale_limits`, `min_visible`, `bounds_mode`, `z_index`, `max_width`, `max_height`, `animation_speed_ms`.
- Signals: `card-moved`, `card-rotated`, `card-scaled`, `card-raised`, `card-folded` on `Card`; `card-added`, `card-removed`, `geometry-changed` on `FloatingBoard`.
- Foldable and closable cards, localized crate strings via Fluent (`i18n/{en,de}/main.ftl`).
- Default card stylesheet (`.floating-card` and related classes, registered with fallback priority).
- `examples/demo.rs`: runnable demo (`cargo run --example demo`) with background content, four cards, dynamic card creation, snapshot/restore and live signal status.

### Fixed

- Dragging a card followed the pointer at roughly half speed and jittered: the drag delta was mapped through `compute_point` under the live (drifting) transform. The delta is now derived from board-space pointer samples taken from event surface coordinates.
- Widgets outside the cards received no pointer input: `smearor-wrot-rotation`'s `RotationWidget` narrows the surface input region to each card's footprint on every allocation (designed for layer-shell surfaces). Cards now opt out via `input_region_enabled(false)` (`smearor-wrot-rotation` 0.2.1); the board keeps a full-region reset as fallback, skippable via `set_input_region_managed`.
- Folding a card shrank the header below the width of its own fold button: the card pins its unfolded natural width via `width-request` while folded.
- Rotating a card left its measured size stale, so a 90 degree turn did not swap content width/height: `notify::rotation` now calls `layout_changed` on the `RotatedLayout` to invalidate the measure chain.
- Replaced the fragile two-gesture pinch/rotate arbitration with a single N-point transform tracker: a raw `EventControllerLegacy` per card observes every touch sequence without claiming (a `GestureSingle` slots design was abandoned - singles lock onto the first sequence and cannot distribute touches across siblings). Centroid delta moves, the RMS point-cloud radius ratio scales uniformly (aspect ratio preserved), the Procrustes angle rotates via `RotationWidget::set_rotation`. Fingers joining/leaving re-snapshot the baseline, ~180° angle jumps re-baseline instead of applying, and touches claimed by other gestures or silent for 1.5 s are swept so lost `TouchEnd` events cannot linger as phantom points. `rotatable`/`scalable`/`draggable` gate transform degrees of freedom; smearor-wrot-rotation's own rotate gesture is disabled.
- Rotation animations (gesture snap and `set_rotation_with_animation`) run 500 ms via `CardConfig::animation_speed_ms`.

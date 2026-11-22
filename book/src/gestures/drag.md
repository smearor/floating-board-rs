# Drag & Raise

The one-finger path: `GestureDrag` on the configured drag surface
moves the card live, `GestureClick` raises it on release.

## Drag surfaces

`CardConfig::drag_handle` picks what the drag gesture attaches to —
the header strip, a dedicated handle icon, or the whole card
(`DragHandle`, see [Chrome](../cards/chrome.md#drag-handles)). The
two corner grips are separate `GestureDrag` controllers: the
**rotate grip** converts the pointer's angular sweep around the card
center into quarter turns in the swipe direction, and the **resize
grip** converts radial distance into `set_scale` within
`scale_limits`.

## Deltas in board space

`GestureDrag` reports offsets in the coordinate space of the drag
source — which lives *inside* the rotated, scaled transform stack and
*moves* with the card. Mapping that delta under the current transform
creates a feedback loop: the reference frame shifts exactly as the
card tracks the pointer, visible as jitter and reduced pointer speed.

The board therefore ignores the reported offset and derives the delta
from two consecutive **board-space pointer samples**: the pointer
position is read from `Gesture::current_event`'s surface coordinates
and mapped through `Native::compute_point` into the card layer — a
frame that never depends on the card's own transform. A pure-vector
fallback (`rotate_vector · scale`) covers the window before the first
allocation.

The same trick applies to the view camera: a pan drag on empty space
reads board-space samples, maps them `to_workspace` and pans the
`View` so the table follows the fingers exactly.

## Raise on release

A `GestureClick` that releases without a drag claims the sequence and
re-appends the card's transform stack on the `gtk4::Fixed` layer —
the child order is the stacking order, so the card pops to the top.
`card-raised` fires; a finished drag raises the same way. One card
serves one active drag sequence at a time, but sequences on
*different* cards are independent — two users can drag two cards
simultaneously.

On drag end (and in `Contained` mode during the drag) the card
re-clamps into the bounds — see
[Bounds & Clamping](../geometry/clamping.md) — and `card-moved` plus
`geometry-changed` fire.

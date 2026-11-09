//! Pure domain model: newtypes, enums, geometry math and layout serde.
//!
//! Nothing in this module depends on `gtk4`; everything is display-free and
//! unit-testable.

mod anchor;
mod board_error;
mod board_layout;
mod card_anchor;
mod card_geometry;
mod card_id;
mod card_placement;
mod card_position;
mod clamp;
mod drag_handle;
mod geometry;
mod min_visible;
mod rect;
mod scale_bounds;
mod scale_factor;

pub use anchor::ANCHOR_MARGIN;
pub use anchor::resolve_anchor;
pub use board_error::BoardError;
pub use board_layout::BoardLayout;
pub use card_anchor::CardAnchor;
pub use card_geometry::CardGeometry;
pub use card_id::CardId;
pub use card_placement::CardPlacement;
pub use card_position::CardPosition;
pub use clamp::clamp_offset;
pub use drag_handle::DragHandle;
pub use geometry::rotate_vector;
pub use geometry::rotated_bounds;
pub use geometry::rotated_region_scanlines;
pub use min_visible::MinVisible;
pub use rect::Rect;
pub use scale_bounds::ScaleBounds;
pub use scale_factor::ScaleFactor;

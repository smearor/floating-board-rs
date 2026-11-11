//! Card-level domain types: identity, geometry, placement anchors, chrome
//! configuration enums and scale/bounds primitives.

mod anchor;
mod bounds;
mod geometry;
mod handle;
mod id;
mod min_visible;
mod placement;
mod position;
mod scale_bounds;
mod scale_factor;

pub use anchor::ANCHOR_MARGIN;
pub use anchor::CardAnchor;
pub use bounds::BoundsMode;
pub use geometry::CardGeometry;
pub use handle::DragHandle;
pub use id::CardId;
pub use min_visible::MinVisible;
pub use placement::CardPlacement;
pub use position::CardPosition;
pub use scale_bounds::ScaleBounds;
pub use scale_factor::ScaleFactor;

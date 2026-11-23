//! Card-level domain types: identity, geometry, placement anchors, chrome
//! configuration enums and scale/bounds primitives.

mod anchor;
mod bounds;
mod chrome_element;
mod chrome_placement;
mod chrome_slot;
mod chrome_style;
mod circle_content;
mod color;
mod geometry;
mod handle;
mod id;
mod min_visible;
mod palette;
mod placement;
mod position;
mod scale_bounds;
mod scale_factor;
mod shape;
mod turn_direction;

pub use anchor::ANCHOR_MARGIN;
pub use anchor::CardAnchor;
pub use bounds::BoundsMode;
pub use chrome_element::ChromeElement;
pub use chrome_placement::ChromePlacement;
pub use chrome_slot::ChromeSlot;
pub use chrome_style::ChromeStyle;
pub use circle_content::CircleContent;
pub use color::CardColor;
pub use geometry::CardGeometry;
pub use handle::DragHandle;
pub use id::CardId;
pub use min_visible::MinVisible;
pub use palette::CardPalette;
pub use placement::CardPlacement;
pub use position::CardPosition;
pub use scale_bounds::ScaleBounds;
pub use scale_factor::ScaleFactor;
pub use shape::CardShape;
pub use turn_direction::TurnDirection;

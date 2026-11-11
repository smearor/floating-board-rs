//! Pure, GTK-free domain core: card identity/geometry, board layout and
//! bounds math, and shared geometry primitives. Everything in this module
//! is serializable or pure value/method code and free of `gtk4` imports.

pub(crate) mod board;
pub(crate) mod card;
pub(crate) mod geometry;

pub use board::BoardError;
pub use board::BoardLayout;
pub use board::SequenceNumber;
pub use card::ANCHOR_MARGIN;
pub use card::BoundsMode;
pub use card::CardAnchor;
pub use card::CardGeometry;
pub use card::CardId;
pub use card::CardPlacement;
pub use card::CardPosition;
pub use card::DragHandle;
pub use card::MinVisible;
pub use card::ScaleBounds;
pub use card::ScaleFactor;
pub use geometry::Rect;

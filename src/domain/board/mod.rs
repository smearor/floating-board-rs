//! Board-level domain types: layout persistence, error type, board-space
//! points and the z-order sequence counter.

mod error;
mod layout;
mod point;
mod sequence;

pub use error::BoardError;
pub use layout::BoardLayout;
pub use point::BoardPoint;
pub use sequence::SequenceNumber;

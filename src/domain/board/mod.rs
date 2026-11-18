//! Board-level domain types: layout persistence, error type, board-space
//! points and the z-order sequence counter.

mod error;
mod generation;
mod goto_rotation;
mod goto_transition;
mod layout;
mod seat_anchor;
mod sequence;

pub use error::BoardError;
pub use generation::GenerationCounter;
pub use goto_rotation::GotoRotation;
pub use goto_transition::GotoTransition;
pub use layout::BoardLayout;
pub use seat_anchor::SeatAnchor;
pub use sequence::SequenceNumber;

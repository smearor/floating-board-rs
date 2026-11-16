use crate::domain::CardId;
use smearor_commons_layout::LayoutError;
use thiserror::Error;

/// Error type of the floating board.
#[derive(Debug, Error)]
pub enum BoardError {
    /// The given card id does not exist on the board.
    #[error("Unknown card: {0}")]
    UnknownCard(CardId),
    /// A card with the given id already exists on the board.
    #[error("Duplicate card: {0}")]
    DuplicateCard(CardId),
    /// A layout strategy could not arrange the cards.
    #[error(transparent)]
    Layout(#[from] LayoutError),
    /// The seat roster holds no seat other than the card's current one.
    #[error("No next seat in the roster for card: {0}")]
    NoNextSeat(CardId),
}

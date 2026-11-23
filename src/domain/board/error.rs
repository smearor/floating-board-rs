use crate::domain::CardId;
use crate::domain::SatelliteId;
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
    /// The given satellite id does not exist on the given card.
    #[error("Unknown satellite {satellite} on card {card}")]
    UnknownSatellite {
        /// The card id.
        card: CardId,
        /// The satellite id.
        satellite: SatelliteId,
    },
    /// An FX snapshot or playback step failed (feature `fx` only).
    #[error("FX operation failed: {0}")]
    Fx(String),
}

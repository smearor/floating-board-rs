use crate::domain::CardId;

/// Error type of the floating board.
#[derive(Debug, thiserror::Error)]
pub enum BoardError {
    /// The given card id does not exist on the board.
    #[error("Unknown card: {0}")]
    UnknownCard(CardId),
    /// A card with the given id already exists on the board.
    #[error("Duplicate card: {0}")]
    DuplicateCard(CardId),
}

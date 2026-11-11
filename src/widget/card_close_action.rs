/// What a card's close button does.
///
/// [`CardCloseAction::Remove`] destroys the card entry outright (the
/// historical behavior). [`CardCloseAction::Hide`] collapses the card
/// stack in place - it keeps its geometry, stays listed via
/// [`crate::FloatingBoard::cards`] and can be re-shown with
/// [`crate::FloatingBoard::set_card_visible`], which suits
/// menu-driven boards where cards are opened and closed repeatedly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CardCloseAction {
    /// The close button removes the card from the board.
    #[default]
    Remove,
    /// The close button hides the card - the same effect as
    /// [`crate::FloatingBoard::set_card_visible`] with `false`.
    Hide,
}

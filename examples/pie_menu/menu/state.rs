//! The pie menu's shared interaction state.

use smearor_commons_color::Color;
use std::cell::Cell;

/// The hovered (`hover`) and last activated (`active`) wedge index -
/// shared between the face's draw func and the card's controllers.
pub(crate) struct MenuState {
    pub(crate) hover: Cell<Option<usize>>,
    pub(crate) active: Cell<Option<usize>>,
    /// The board background color - repainted on every activation.
    pub(crate) background: Cell<Color<f64>>,
}

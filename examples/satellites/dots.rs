//! The color-dot satellites: buttons that re-color their card via
//! `Card::set_palette`.

use floating_board::CardColor;
use floating_board::CardId;
use floating_board::CardPalette;
use floating_board::FloatingBoard;
use gtk4::Button;
use gtk4::prelude::*;
use smearor_commons_color::Color;

/// A circular color-dot satellite button styled by `class`.
pub(crate) fn dot_button(class: &str) -> Button {
    let button = Button::new();
    button.add_css_class(class);
    button.add_css_class("memo-dot");
    button
}

/// Wires a color-dot satellite to re-color `card_id` on click via
/// `Card::set_palette`.
pub(crate) fn wire_color_dot(board: &FloatingBoard, card_id: &CardId, button: &Button, background: DotColor, header: DotColor) {
    let board = board.clone();
    let id = card_id.clone();
    button.connect_clicked(move |_| {
        if let Ok(card) = board.card(&id) {
            let mut palette = CardPalette::new();
            palette.insert(CardColor::Background, background);
            palette.insert(CardColor::Header, header);
            palette.insert(CardColor::Foreground, INK);
            card.set_palette(&palette);
        }
    });
}

/// A palette color for the dot specs.
pub(crate) type DotColor = Color<f64>;

/// Dot colors as (css class, background, header) - shared by both
/// cards so the same dot picks the same color.
pub(crate) const DOT_SPECS: [(&str, DotColor, DotColor); 3] = [
    ("memo-green", Color::<f64>::new(0.20, 0.83, 0.48), Color::<f64>::new(0.13, 0.60, 0.34)),
    ("memo-red", Color::<f64>::new(0.94, 0.26, 0.21), Color::<f64>::new(0.72, 0.14, 0.12)),
    ("memo-yellow", Color::<f64>::new(0.96, 0.83, 0.18), Color::<f64>::new(0.86, 0.68, 0.10)),
];

/// The memo cards' shared foreground ink - the dots' recolor and the
/// memo palette agree on it.
pub(crate) const INK: Color<f64> = Color::<f64>::new(0.20, 0.18, 0.10);

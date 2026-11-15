use crate::domain::card::color::CardColor;
use smearor_commons_color::Color;
use smearor_commons_color::Palette;

/// Semantic color set of a card: a [`Palette`] mapping [`CardColor`]
/// slots to normalized colors.
///
/// Passed via `CardConfig::palette`; the card turns it into a scoped
/// CSS provider at build time. Slots without a registered color keep
/// the themed or default-card colors, so a palette may fill any
/// subset.
///
/// ```
/// use floating_board::CardColor;
/// use floating_board::CardPalette;
/// use smearor_commons_color::Color;
///
/// let mut palette = CardPalette::new();
/// palette.insert(CardColor::Background, Color::<f64>::new(0.2, 0.3, 0.4));
/// palette.insert(CardColor::Foreground, Color::<f64>::new(0.9, 0.9, 0.9));
/// ```
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(transparent))]
pub struct CardPalette(Palette<CardColor, f64>);

impl CardPalette {
    /// An empty palette - every slot keeps its themed color.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `color` for `slot`, returning the replaced color.
    pub fn insert(&mut self, slot: CardColor, color: Color<f64>) -> Option<Color<f64>> {
        self.0.insert(slot, color)
    }

    /// The color registered for `slot`, or `None`.
    pub fn get(&self, slot: CardColor) -> Option<Color<f64>> {
        self.0.get(&slot).copied()
    }

    /// The number of filled slots.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` when no slot carries a color - such a palette changes
    /// nothing and is skipped when building the card.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The slots declared by [`CardColor`] that have no color
    /// registered, in declaration order.
    pub fn missing(&self) -> Vec<CardColor> {
        self.0.missing()
    }
}

impl From<Palette<CardColor, f64>> for CardPalette {
    fn from(palette: Palette<CardColor, f64>) -> Self {
        Self(palette)
    }
}

impl From<CardPalette> for Palette<CardColor, f64> {
    fn from(palette: CardPalette) -> Self {
        palette.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserted_colors_are_found_and_missing_slots_reported() {
        let mut palette = CardPalette::new();
        palette.insert(CardColor::Background, Color::<f64>::new(0.5, 0.5, 0.5));
        assert_eq!(palette.get(CardColor::Background), Some(Color::<f64>::new(0.5, 0.5, 0.5)));
        assert_eq!(palette.get(CardColor::Foreground), None);
        assert_eq!(palette.missing(), vec![CardColor::Foreground, CardColor::Header]);
    }
}

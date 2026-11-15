use smearor_commons_color::PaletteDefinition;

/// The semantic color slots of a card - the vocabulary of a
/// [`CardPalette`](crate::domain::CardPalette).
///
/// Every slot maps to one CSS property of the card's chrome; slots left
/// unfilled keep the themed or default-card color. Written by hand
/// instead of `smearor_commons_color::palette_definition!` because the
/// macro derives `serde` unconditionally while this crate gates serde
/// behind a feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum CardColor {
    /// Card background (`background-color` of `.floating-card`).
    Background,
    /// Text and icon color (`color` of `.floating-card`) - covers the
    /// title, the header buttons and the grip strokes.
    Foreground,
    /// Header strip background (`background-color` of
    /// `.floating-card-header`).
    Header,
}

impl PaletteDefinition for CardColor {
    fn slots() -> &'static [Self] {
        &[Self::Background, Self::Foreground, Self::Header]
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Foreground => "foreground",
            Self::Header => "header",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_lists_all_variants_in_declaration_order() {
        assert_eq!(CardColor::slots(), &[CardColor::Background, CardColor::Foreground, CardColor::Header]);
    }

    #[test]
    fn name_returns_the_css_role() {
        assert_eq!(CardColor::Background.name(), "background");
        assert_eq!(CardColor::Foreground.name(), "foreground");
        assert_eq!(CardColor::Header.name(), "header");
    }
}

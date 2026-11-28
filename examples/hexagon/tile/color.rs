//! The `tile` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The game tile's named colors.
    pub enum TileColor {
        /// The sand-colored tile face.
        Face => "face",
        /// The darker rim stroke around the hexagon.
        Rim => "rim",
        /// The light number-token disc.
        Token => "token",
        /// The token's value and pips - one shared ink.
        Ink => "ink"
    }
}

/// The sRGB values of the [`TileColor`] slots.
static TILE_PALETTE: StaticPalette<TileColor, f64> = static_palette! {
    TileColor::Face => Color::<f64>::new(0.85, 0.71, 0.40),
    TileColor::Rim => Color::<f64>::new(0.42, 0.30, 0.12),
    TileColor::Token => Color::<f64>::new(0.96, 0.93, 0.86),
    TileColor::Ink => Color::<f64>::new(0.20, 0.16, 0.10),
};

/// The color registered under `slot` in the tile palette - black on
/// a miss, unreachable while the definition stays complete.
pub(super) fn tile_color(slot: TileColor) -> Color<f64> {
    TILE_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

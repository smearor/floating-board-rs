//! The `pie` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The pie chart's named colors - each is both a slice fill and
    /// the matching satellite dot's color.
    pub enum PieColor {
        /// The blue slice and satellite.
        Blue => "blue",
        /// The green slice and satellite.
        Green => "green",
        /// The yellow slice and satellite.
        Yellow => "yellow",
        /// The red slice and satellite - the slice starts absent.
        Red => "red"
    }
}

/// The sRGB values of the [`PieColor`] slots.
static PIE_PALETTE: StaticPalette<PieColor, f64> = static_palette! {
    PieColor::Blue => Color::<f64>::new(0.32, 0.62, 0.84),
    PieColor::Green => Color::<f64>::new(0.24, 0.72, 0.48),
    PieColor::Yellow => Color::<f64>::new(0.92, 0.66, 0.20),
    PieColor::Red => Color::<f64>::new(0.85, 0.25, 0.22),
};

/// The color registered under `slot` in the pie palette - black on a
/// miss, unreachable while the definition stays complete.
pub(super) fn pie_color(slot: PieColor) -> Color<f64> {
    PIE_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

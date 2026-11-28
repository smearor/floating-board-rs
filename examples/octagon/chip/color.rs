//! The `chip` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The casino plaque's named colors.
    pub enum ChipColor {
        /// The deep green felt face.
        Felt => "felt",
        /// The pale rim stroke around the octagon.
        Rim => "rim",
        /// The light wedge inserts on the eight edge midpoints.
        Wedge => "wedge",
        /// The denomination lettering.
        Denomination => "denomination"
    }
}

/// The sRGB values of the [`ChipColor`] slots.
static CHIP_PALETTE: StaticPalette<ChipColor, f64> = static_palette! {
    ChipColor::Felt => Color::<f64>::new(0.10, 0.32, 0.20),
    ChipColor::Rim => Color::<f64>::new(0.85, 0.80, 0.55),
    ChipColor::Wedge => Color::<f64>::new(0.90, 0.88, 0.78),
    ChipColor::Denomination => Color::<f64>::new(0.93, 0.92, 0.85),
};

/// The color registered under `slot` in the chip palette - black on
/// a miss, unreachable while the definition stays complete.
pub(super) fn chip_color(slot: ChipColor) -> Color<f64> {
    CHIP_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

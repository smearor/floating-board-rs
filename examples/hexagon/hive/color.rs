//! The `hive` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The honeycomb patch's named colors.
    pub enum HiveColor {
        /// The painted square under the cells - the card face shows
        /// through the clipped corner triangles.
        Backdrop => "backdrop",
        /// The honeycomb rim stroke around every cell.
        Honey => "honey",
        /// The queen cell in the middle.
        Center => "center"
    }
}

/// The sRGB values of the [`HiveColor`] slots.
static HIVE_PALETTE: StaticPalette<HiveColor, f64> = static_palette! {
    HiveColor::Backdrop => Color::<f64>::new(0.24, 0.17, 0.05),
    HiveColor::Honey => Color::<f64>::new(0.95, 0.77, 0.28),
    HiveColor::Center => Color::<f64>::new(0.55, 0.38, 0.10),
};

/// The color registered under `slot` in the hive palette - black on
/// a miss, unreachable while the definition stays complete.
pub(super) fn hive_color(slot: HiveColor) -> Color<f64> {
    HIVE_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

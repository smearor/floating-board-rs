//! The `bezel` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The bezel dial's named colors.
    pub enum BezelColor {
        /// The dark square face under the clipped tick ring.
        Face => "face",
        /// The bright tick marks.
        Ticks => "ticks"
    }
}

/// The sRGB values of the [`BezelColor`] slots.
static BEZEL_PALETTE: StaticPalette<BezelColor, f64> = static_palette! {
    BezelColor::Face => Color::<f64>::new(0.22, 0.32, 0.42),
    BezelColor::Ticks => Color::<f64>::new(0.90, 0.92, 0.95),
};

/// The color registered under `slot` in the bezel palette - black
/// on a miss, unreachable while the definition stays complete.
pub(super) fn bezel_color(slot: BezelColor) -> Color<f64> {
    BEZEL_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

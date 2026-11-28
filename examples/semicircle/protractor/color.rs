//! The `protractor` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The protractor's named colors.
    pub enum ProtractorColor {
        /// The warm acrylic face.
        Face => "face",
        /// The bronze markings: rim, ticks, scale arc, origin mark.
        Bronze => "bronze",
        /// The dark half-disc cutout on the flat edge.
        Cutout => "cutout",
        /// The red swing arm and pivot hub.
        Needle => "needle"
    }
}

/// The sRGB values of the [`ProtractorColor`] slots.
static PROTRACTOR_PALETTE: StaticPalette<ProtractorColor, f64> = static_palette! {
    ProtractorColor::Face => Color::<f64>::new(0.95, 0.86, 0.55),
    ProtractorColor::Bronze => Color::<f64>::new(0.55, 0.42, 0.18),
    ProtractorColor::Cutout => Color::<f64>::new(0.24, 0.20, 0.28),
    ProtractorColor::Needle => Color::<f64>::new(0.76, 0.26, 0.20),
};

/// The color registered under `slot` in the protractor palette -
/// black on a miss, unreachable while the definition stays complete.
pub(super) fn protractor_color(slot: ProtractorColor) -> Color<f64> {
    PROTRACTOR_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

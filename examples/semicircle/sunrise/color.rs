//! The `sunrise` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The sunrise's named colors.
    pub enum SunriseColor {
        /// The rising sun and its rays.
        Sun => "sun",
        /// The horizon line the sun rises over.
        Horizon => "horizon"
    }
}

/// The sRGB values of the [`SunriseColor`] slots.
static SUNRISE_PALETTE: StaticPalette<SunriseColor, f64> = static_palette! {
    SunriseColor::Sun => Color::<f64>::new(0.98, 0.78, 0.30),
    SunriseColor::Horizon => Color::<f64>::new(0.45, 0.38, 0.55),
};

/// The color registered under `slot` in the sunrise palette - black
/// on a miss, unreachable while the definition stays complete.
pub(super) fn sunrise_color(slot: SunriseColor) -> Color<f64> {
    SUNRISE_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

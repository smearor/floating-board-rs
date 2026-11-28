//! The `compass` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The compass rose's named colors.
    pub enum CompassColor {
        /// The square under-paint for the clipped corner triangles.
        Backdrop => "backdrop",
        /// The night-blue face.
        Face => "face",
        /// The steel-blue rim stroke around the octagon.
        Rim => "rim",
        /// The cardinal arms (N/E/S/W).
        Cardinal => "cardinal",
        /// The intercardinal arms (NE/SE/SW/NW).
        Intercardinal => "intercardinal",
        /// The center hub and the north label - one shared white.
        Hub => "hub"
    }
}

/// The sRGB values of the [`CompassColor`] slots.
static COMPASS_PALETTE: StaticPalette<CompassColor, f64> = static_palette! {
    CompassColor::Backdrop => Color::<f64>::new(0.10, 0.15, 0.24),
    CompassColor::Face => Color::<f64>::new(0.13, 0.20, 0.32),
    CompassColor::Rim => Color::<f64>::new(0.55, 0.67, 0.80),
    CompassColor::Cardinal => Color::<f64>::new(0.91, 0.35, 0.32),
    CompassColor::Intercardinal => Color::<f64>::new(0.82, 0.86, 0.90),
    CompassColor::Hub => Color::<f64>::new(0.95, 0.95, 0.96),
};

/// The color registered under `slot` in the compass palette - black
/// on a miss, unreachable while the definition stays complete.
pub(super) fn compass_color(slot: CompassColor) -> Color<f64> {
    COMPASS_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

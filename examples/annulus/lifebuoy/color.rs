//! The `lifebuoy` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The lifebuoy's named colors.
    pub enum LifebuoyColor {
        /// The faint disc showing the square's extent over the hole.
        Face => "face",
        /// The red band stripes.
        Red => "red",
        /// The white band stripes.
        White => "white",
        /// The dark rim stroke around the disc.
        Rim => "rim"
    }
}

/// The sRGB values of the [`LifebuoyColor`] slots.
static LIFEBUOY_PALETTE: StaticPalette<LifebuoyColor, f64> = static_palette! {
    LifebuoyColor::Face => Color::<f64>::new(0.96, 0.96, 0.98),
    LifebuoyColor::Red => Color::<f64>::new(0.85, 0.20, 0.18),
    LifebuoyColor::White => Color::<f64>::new(0.95, 0.94, 0.90),
    LifebuoyColor::Rim => Color::<f64>::new(0.55, 0.10, 0.08),
};

/// The color registered under `slot` in the lifebuoy palette -
/// black on a miss, unreachable while the definition stays complete.
pub(super) fn lifebuoy_color(slot: LifebuoyColor) -> Color<f64> {
    LIFEBUOY_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

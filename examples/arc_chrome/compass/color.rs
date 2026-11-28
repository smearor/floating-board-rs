//! The `compass` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The compass dial's named colors.
    pub enum CompassColor {
        /// The dial's dark face, painted at 95% opacity.
        Face => "face",
        /// The bright lines on the dial: ring, ticks, hub and needle tails.
        Line => "line",
        /// The card-fixed needle - always points to the card's own top edge.
        CardNeedle => "card-needle",
        /// The world-fixed needle - always points to the board's top edge.
        WorldNeedle => "world-needle"
    }
}

/// The sRGB values of the [`CompassColor`] slots.
static COMPASS_PALETTE: StaticPalette<CompassColor, f64> = static_palette! {
    CompassColor::Face => Color::<f64>::new(0.16, 0.24, 0.34),
    CompassColor::Line => Color::<f64>::new(0.85, 0.88, 0.92),
    CompassColor::CardNeedle => Color::<f64>::new(0.18, 0.72, 0.36),
    CompassColor::WorldNeedle => Color::<f64>::new(0.87, 0.2, 0.25),
};

/// The color registered under `slot` in the compass palette - black
/// on a miss, unreachable while the definition stays complete.
pub(super) fn compass_color(slot: CompassColor) -> Color<f64> {
    COMPASS_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

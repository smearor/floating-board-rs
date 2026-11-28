//! The `gauge` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The dome face's named colors.
    pub enum DomeColor {
        /// The dark dial face, painted at 95% opacity.
        Face => "face",
        /// The bright markings: rim, ticks, hub.
        Line => "line",
        /// The gauge needle.
        Needle => "needle"
    }
}

/// The sRGB values of the [`DomeColor`] slots.
static DOME_PALETTE: StaticPalette<DomeColor, f64> = static_palette! {
    DomeColor::Face => Color::<f64>::new(0.16, 0.22, 0.32),
    DomeColor::Line => Color::<f64>::new(0.85, 0.88, 0.92),
    DomeColor::Needle => Color::<f64>::new(0.87, 0.35, 0.28),
};

/// The color registered under `slot` in the dome palette - black on a
/// miss, unreachable while the definition stays complete.
pub(super) fn dome_color(slot: DomeColor) -> Color<f64> {
    DOME_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

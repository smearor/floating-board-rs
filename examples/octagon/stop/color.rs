//! The `stop` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The stop sign's named colors.
    pub enum StopColor {
        /// The red sign face.
        Face => "face",
        /// The white markings - rim, inner ring and lettering share
        /// one white.
        Marking => "marking"
    }
}

/// The sRGB values of the [`StopColor`] slots.
static STOP_PALETTE: StaticPalette<StopColor, f64> = static_palette! {
    StopColor::Face => Color::<f64>::new(0.72, 0.11, 0.11),
    StopColor::Marking => Color::<f64>::new(0.96, 0.96, 0.97),
};

/// The color registered under `slot` in the stop palette - black on
/// a miss, unreachable while the definition stays complete.
pub(super) fn stop_color(slot: StopColor) -> Color<f64> {
    STOP_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

//! The `clock` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The clock face's named colors - face, the shared ticks/hands
    /// ink and the seconds accent.
    pub enum ClockColor {
        /// The light face disc.
        Face => "face",
        /// Ticks, hour hand and minute hand - one shared ink.
        Ink => "ink",
        /// The seconds hand and hub accent.
        Accent => "accent"
    }
}

/// The sRGB values of the [`ClockColor`] slots.
static CLOCK_PALETTE: StaticPalette<ClockColor, f64> = static_palette! {
    ClockColor::Face => Color::<f64>::new(0.95, 0.95, 0.97),
    ClockColor::Ink => Color::<f64>::new(0.22, 0.27, 0.36),
    ClockColor::Accent => Color::<f64>::new(0.87, 0.2, 0.25),
};

/// The color registered under `slot` in the clock palette - black on
/// a miss, unreachable while the definition stays complete.
pub(super) fn clock_color(slot: ClockColor) -> Color<f64> {
    CLOCK_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

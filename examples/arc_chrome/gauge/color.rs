//! The `gauge` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The gauge dial's named colors - face, level ring parts and
    /// the shared needle/hub ink.
    pub enum GaugeColor {
        /// The dial's dark face disc.
        Face => "face",
        /// The dim level-ring track under the fill arc.
        Track => "track",
        /// The bright level fill arc.
        Level => "level",
        /// The tick ring inside the level ring.
        Ticks => "ticks",
        /// The needle line and the hub dot - one shared ink.
        Ink => "ink"
    }
}

/// The sRGB values of the [`GaugeColor`] slots.
static GAUGE_PALETTE: StaticPalette<GaugeColor, f64> = static_palette! {
    GaugeColor::Face => Color::<f64>::new(0.12, 0.14, 0.18),
    GaugeColor::Track => Color::<f64>::new(0.42, 0.48, 0.60),
    GaugeColor::Level => Color::<f64>::new(0.35, 0.78, 0.56),
    GaugeColor::Ticks => Color::<f64>::new(0.85, 0.88, 0.95),
    GaugeColor::Ink => Color::<f64>::new(0.92, 0.93, 0.96),
};

/// The color registered under `slot` in the gauge palette - black on
/// a miss, unreachable while the definition stays complete.
pub(super) fn gauge_color(slot: GaugeColor) -> Color<f64> {
    GAUGE_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

//! The `gem` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The brilliant-cut gem's named colors.
    pub enum GemColor {
        /// The crown facets' base hue - the per-facet shade scales
        /// it multiplicatively.
        Crown => "crown",
        /// The crown facets' dark rim stroke.
        CrownRim => "crown-rim",
        /// The light table facet.
        Table => "table",
        /// The table's rim stroke.
        TableRim => "table-rim"
    }
}

/// The sRGB values of the [`GemColor`] slots.
static GEM_PALETTE: StaticPalette<GemColor, f64> = static_palette! {
    GemColor::Crown => Color::<f64>::new(0.15, 0.72, 0.78),
    GemColor::CrownRim => Color::<f64>::new(0.05, 0.20, 0.22),
    GemColor::Table => Color::<f64>::new(0.55, 0.92, 0.94),
    GemColor::TableRim => Color::<f64>::new(0.08, 0.30, 0.33),
};

/// The color registered under `slot` in the gem palette - black on
/// a miss, unreachable while the definition stays complete.
pub(super) fn gem_color(slot: GemColor) -> Color<f64> {
    GEM_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

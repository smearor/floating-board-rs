//! The `donut` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The donut's named colors.
    pub enum DonutColor {
        /// The painted square corners escaping into the hole.
        Backdrop => "backdrop",
        /// The golden dough slices.
        Dough => "dough",
        /// The dark cocoa glaze slices.
        Cocoa => "cocoa",
        /// The pale cream glaze slices.
        Cream => "cream"
    }
}

/// The sRGB values of the [`DonutColor`] slots.
static DONUT_PALETTE: StaticPalette<DonutColor, f64> = static_palette! {
    DonutColor::Backdrop => Color::<f64>::new(0.40, 0.28, 0.14),
    DonutColor::Dough => Color::<f64>::new(0.93, 0.62, 0.30),
    DonutColor::Cocoa => Color::<f64>::new(0.60, 0.36, 0.18),
    DonutColor::Cream => Color::<f64>::new(0.98, 0.80, 0.50),
};

/// The color registered under `slot` in the donut palette - black
/// on a miss, unreachable while the definition stays complete.
pub(super) fn donut_color(slot: DonutColor) -> Color<f64> {
    DONUT_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

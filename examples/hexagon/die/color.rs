//! The `die` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The die face's named colors.
    pub enum DieColor {
        /// The square under-paint reaching into the corner
        /// triangles the `Fill` allocation exposes.
        Backdrop => "backdrop",
        /// The dark die face.
        Face => "face",
        /// The pale hexagon rim stroke.
        Rim => "rim",
        /// The pips.
        Pip => "pip"
    }
}

/// The sRGB values of the [`DieColor`] slots.
static DIE_PALETTE: StaticPalette<DieColor, f64> = static_palette! {
    DieColor::Backdrop => Color::<f64>::new(0.08, 0.09, 0.12),
    DieColor::Face => Color::<f64>::new(0.13, 0.14, 0.19),
    DieColor::Rim => Color::<f64>::new(0.55, 0.58, 0.68),
    DieColor::Pip => Color::<f64>::new(0.92, 0.93, 0.97),
};

/// The color registered under `slot` in the die palette - black on
/// a miss, unreachable while the definition stays complete.
pub(super) fn die_color(slot: DieColor) -> Color<f64> {
    DIE_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

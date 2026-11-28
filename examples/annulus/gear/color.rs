//! The `gear` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The gear's named colors.
    pub enum GearColor {
        /// The bright machined teeth ring.
        Teeth => "teeth",
        /// The spokes crossing the band.
        Spoke => "spoke",
        /// The hub ring around the hole.
        Hub => "hub"
    }
}

/// The sRGB values of the [`GearColor`] slots.
static GEAR_PALETTE: StaticPalette<GearColor, f64> = static_palette! {
    GearColor::Teeth => Color::<f64>::new(0.62, 0.63, 0.66),
    GearColor::Spoke => Color::<f64>::new(0.45, 0.46, 0.50),
    GearColor::Hub => Color::<f64>::new(0.30, 0.30, 0.33),
};

/// The color registered under `slot` in the gear palette - black on
/// a miss, unreachable while the definition stays complete.
pub(super) fn gear_color(slot: GearColor) -> Color<f64> {
    GEAR_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

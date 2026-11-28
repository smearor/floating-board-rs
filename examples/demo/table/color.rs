//! The `table` face's color palette.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The table face's named colors - surface, grid and the shared
    /// hint/letter ink. The user marker fills live in `USER_COLORS`:
    /// they cycle positionally and are a different concern.
    pub enum TableColor {
        /// The dark table surface under everything.
        Surface => "surface",
        /// The 80px background grid lines.
        Grid => "grid",
        /// The shared ink: gesture hint and marker letters.
        Ink => "ink"
    }
}

/// The sRGB values of the [`TableColor`] slots.
static TABLE_PALETTE: StaticPalette<TableColor, f64> = static_palette! {
    TableColor::Surface => Color::<f64>::new(0.16, 0.18, 0.22),
    TableColor::Grid => Color::<f64>::new(0.25, 0.28, 0.33),
    TableColor::Ink => Color::<f64>::new(1.0, 1.0, 1.0),
};

/// The color registered under `slot` in the table palette - black on
/// a miss, unreachable while the definition stays complete.
pub(super) fn table_color(slot: TableColor) -> Color<f64> {
    TABLE_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

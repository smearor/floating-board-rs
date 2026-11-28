//! The pie menu's color palettes.

use smearor_commons_color::Color;
use smearor_commons_color::StaticPalette;
use smearor_commons_color::palette_definition;
use smearor_commons_color::static_palette;

palette_definition! {
    /// The pie menu's wedge colors - one slot per menu entry, in
    /// ring order starting at the top and going clockwise.
    pub enum WedgeColor {
        /// The Cut wedge - a muted red.
        Cut => "cut",
        /// The Copy wedge - a steel blue.
        Copy => "copy",
        /// The Paste wedge - a soft green.
        Paste => "paste",
        /// The Undo wedge - a warm amber.
        Undo => "undo",
        /// The Rotate wedge - a desaturated purple.
        Rotate => "rotate",
        /// The Zoom wedge - a teal.
        Zoom => "zoom",
        /// The Pin wedge - a terracotta.
        Pin => "pin",
        /// The Info wedge - a neutral slate.
        Info => "info"
    }
}

/// The sRGB values of the [`WedgeColor`] slots.
static WEDGE_PALETTE: StaticPalette<WedgeColor, f64> = static_palette! {
    WedgeColor::Cut => Color::<f64>::new(0.72, 0.30, 0.32),
    WedgeColor::Copy => Color::<f64>::new(0.33, 0.56, 0.75),
    WedgeColor::Paste => Color::<f64>::new(0.38, 0.68, 0.45),
    WedgeColor::Undo => Color::<f64>::new(0.82, 0.64, 0.28),
    WedgeColor::Rotate => Color::<f64>::new(0.60, 0.42, 0.72),
    WedgeColor::Zoom => Color::<f64>::new(0.30, 0.62, 0.68),
    WedgeColor::Pin => Color::<f64>::new(0.80, 0.46, 0.36),
    WedgeColor::Info => Color::<f64>::new(0.48, 0.52, 0.58),
};

/// The color registered under `slot` in the wedge palette - black
/// on a miss, unreachable while the definition stays complete.
pub(crate) fn wedge_color(slot: WedgeColor) -> Color<f64> {
    WEDGE_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

palette_definition! {
    /// The pie menu's chrome colors - the shared ink, rim rings and
    /// the board background behind the menu card.
    pub enum MenuColor {
        /// The near-white ink of labels and the hovered-wedge outline.
        Ink => "ink",
        /// The dark ring lines tightening the band's silhouette edges.
        Rim => "rim",
        /// The neutral board background before the first activation.
        Background => "background"
    }
}

/// The sRGB values of the [`MenuColor`] slots.
static MENU_PALETTE: StaticPalette<MenuColor, f64> = static_palette! {
    MenuColor::Ink => Color::<f64>::new(0.97, 0.97, 0.99),
    MenuColor::Rim => Color::<f64>::new(0.15, 0.17, 0.22),
    MenuColor::Background => Color::<f64>::new(0.15, 0.20, 0.24),
};

/// The color registered under `slot` in the chrome palette - black
/// on a miss, unreachable while the definition stays complete.
pub(crate) fn menu_color(slot: MenuColor) -> Color<f64> {
    MENU_PALETTE.get_or(&slot, Color::<f64>::BLACK)
}

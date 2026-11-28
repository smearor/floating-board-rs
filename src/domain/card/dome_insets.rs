use smearor_commons_geometry::Distance;
use smearor_commons_geometry::Pixels;

/// The chrome insets shaping a `Semicircle` card's dome: `ring`
/// widens the inner dome the content must fit (the card's own edge
/// band plus the arc-chrome band when one is mounted), `bottom_inset`
/// lifts the content off the flat edge (the straight bar's band
/// width). Folded cards keep their full dome - no chrome applies.
///
/// Both fields travel together through [`CardContent::dome_radius_for`]
/// and [`CardContent::dome_content_for`]; the named struct keeps them
/// unswappable at call sites.
///
/// [`CardContent::dome_radius_for`]: crate::domain::CardContent::dome_radius_for
/// [`CardContent::dome_content_for`]: crate::domain::CardContent::dome_content_for
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DomeInsets {
    /// The band width inside the dome's curved edge (px).
    pub ring: Distance<Pixels>,
    /// The height the content lifts off the dome's flat bottom edge (px).
    pub bottom_inset: Distance<Pixels>,
}

impl DomeInsets {
    /// Insets of `ring` band width and `bottom_inset` lift.
    pub fn new(ring: Distance<Pixels>, bottom_inset: Distance<Pixels>) -> Self {
        Self { ring, bottom_inset }
    }
}

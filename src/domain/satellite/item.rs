use crate::domain::SatelliteId;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rotation;
use smearor_commons_geometry::Vec2;

/// One satellite as resolver input: identity plus the size its widget
/// measured, in card-local `Pixels`.
///
/// The widget layer builds items from the satellite configs with the
/// widget's natural size - keeping the resolver GTK-free.
#[derive(Debug, Clone, PartialEq)]
pub struct SatelliteItem {
    /// Stable id of the satellite this item places.
    pub id: SatelliteId,
    /// Measured natural size (width x height) in `Pixels`.
    pub size: Vec2<Pixels>,
    /// Fixed anchor angle honored by `SatelliteDistribution::Anchored`
    /// (a `QuarterTurn` anchor is `Rotation::QuarterTurn(..)`); `None`
    /// = flexible item distributed by the layout. Radial-matter -
    /// ignored by the sidebar path.
    pub anchor: Option<Rotation>,
}

impl SatelliteItem {
    /// A flexible item (no fixed anchor) of the measured `size`.
    pub fn new(id: SatelliteId, size: Vec2<Pixels>) -> Self {
        Self { id, size, anchor: None }
    }
}

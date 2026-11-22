use crate::domain::CardGeometry;
use crate::domain::SatelliteId;
use crate::widget::scaling::ScalingWidget;
use gtk4::Widget;
use smearor_commons_geometry::Rotation;
use smearor_wrot_rotation::RotationWidget;
use std::cell::Cell;

/// Board-owned runtime state of one mounted satellite: the same mini
/// transform stack a card gets (`ScalingWidget` on the `CardLayer` ->
/// `RotationWidget` -> satellite widget), so view zoom, card rotation
/// and card scale apply through the existing machinery.
pub(crate) struct SatelliteEntry {
    /// Stable id - keys the resolver's `SatellitePlacement` results.
    pub id: SatelliteId,
    /// Fixed anchor angle for `SatelliteDistribution::Anchored`
    /// (radial matter, Phase 3).
    pub anchor: Option<Rotation>,
    /// The satellite's own widget (chrome button or consumer widget).
    pub widget: Widget,
    /// Outer transform: scale wrapper placed on the `gtk4::Fixed` layer.
    pub scaling: ScalingWidget,
    /// Inner transform: rotation container holding `widget`.
    pub rotation_widget: RotationWidget,
    /// The last applied workspace placement - `None` until the first
    /// sync. The clamp union footprints visible satellites from it.
    pub last_geometry: Cell<Option<CardGeometry>>,
    /// Runtime open flag (`CardSatellites::open` at mount,
    /// `FloatingBoard::set_satellites_open` later): a closed satellite
    /// hides like a folded card hides its set. The *animation* of
    /// opening/closing is Phase-5 matter.
    pub open: Cell<bool>,
}

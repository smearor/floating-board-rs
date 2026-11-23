use crate::domain::CardGeometry;
use crate::domain::SatelliteId;
use crate::domain::SatelliteKind;
use crate::widget::scaling::ScalingWidget;
use gtk4::Widget;
use smearor_commons_animation::Span;
use smearor_commons_animation::TimedAnimation;
use smearor_commons_geometry::Rotation;
use smearor_wrot_rotation::RotationWidget;
use std::cell::Cell;
use std::cell::RefCell;

/// Board-owned runtime state of one mounted satellite: the same mini
/// transform stack a card gets (`ScalingWidget` on the `CardLayer` ->
/// `RotationWidget` -> satellite widget), so view zoom, card rotation
/// and card scale apply through the existing machinery.
pub(crate) struct SatelliteEntry {
    /// Stable id - keys the resolver's `SatellitePlacement` results.
    pub id: SatelliteId,
    /// The satellite's kind - distinguishes chrome elements from
    /// consumer widgets (needed for the circle card's fold exception:
    /// its `Minimize` satellite is the only unfold control).
    pub kind: SatelliteKind,
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
    /// In-flight open/close/move transition (Phase 5). While set, the
    /// applied placement interpolates `from` (or the collapsed seed)
    /// toward the live resolved placement (or back into the seed).
    pub transition: RefCell<Option<SatelliteTransition>>,
    /// The resolved placement expressed in card-local space at the last
    /// sync, used to distinguish "card moved" (rigid follow - no tween)
    /// from "placement retargeted" (footprint/layout change - tween).
    pub last_local_geometry: Cell<Option<CardGeometry>>,
}

/// An in-flight satellite placement transition (open, close or
/// retarget). Both endpoints resolve live each frame where possible so
/// card gestures keep carrying an animating satellite.
pub(crate) struct SatelliteTransition {
    /// Eased progress driver; staggered starts come from
    /// `TimedAnimation::start_at` offsets.
    pub animation: TimedAnimation<Span<f64>>,
    /// Placement frozen at transition start (mid-flight reversal or a
    /// retarget). `None` means emerge from the collapsed seed.
    pub from: Option<CardGeometry>,
    /// `true`: the end point is the live resolved placement (open /
    /// retarget). `false`: the end point is the collapsed seed and the
    /// satellite hides when the animation finishes (close).
    pub opening: bool,
}

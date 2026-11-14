use crate::domain::CardAnchor;
use crate::domain::CardPosition;
use crate::domain::ScaleBounds;
use crate::domain::ScaleFactor;
use crate::domain::SequenceNumber;
use crate::widget::card::Card;
use crate::widget::card_config::CardConfig;
use crate::widget::imp::quad_baseline::QuadBaseline;
use crate::widget::imp::tracked_touch::TrackedTouch;
use crate::widget::scaling::ScalingWidget;
use gtk4::gdk::EventSequence;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use smearor_wrot_rotation::RotationWidget;
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Instant;

/// Board-owned state of one card: the transform stack widgets plus the
/// geometry cells. `Card` objects carry only a weak backlink - this entry is
/// the single owner of position/scale state (rotation lives inside the
/// `RotationWidget`'s layout and is read on demand).
pub(crate) struct CardEntry {
    /// The card view widget.
    pub card: Card,
    /// Outer transform: scale wrapper placed on the `gtk4::Fixed` layer.
    pub scaling: ScalingWidget,
    /// Inner transform: rotation container holding the card.
    pub rotation_widget: RotationWidget,
    /// Immutable configuration passed at `add_card`.
    pub config: CardConfig,
    /// Current center position in board coordinates.
    pub position: Cell<CardPosition>,
    /// Current scale factor.
    pub scale: Cell<ScaleFactor>,
    /// Anchor awaiting resolution at the next board allocation.
    pub pending_anchor: RefCell<Option<CardAnchor>>,
    /// Card center at the start of the active drag gesture.
    pub drag_start: Cell<CardPosition>,
    /// Pointer position in board coordinates at drag begin.
    pub drag_grab: Cell<Option<Vec2<Pixels>>>,
    /// Pointer position in board coordinates at resize-drag begin;
    /// `Some` while a resize gesture on the card's grip is active.
    pub resize_grab: Cell<Option<Vec2<Pixels>>>,
    /// Scale factor captured at resize-drag begin; updates apply the
    /// ratio between the current and the grab radial distance.
    pub resize_scale: Cell<ScaleFactor>,
    /// Live touch points for the quad transform, one entry per finger
    /// currently on the card. The tracker's `EventControllerLegacy` keeps
    /// this in sync - a plain event observer that sees every sequence
    /// without claiming it, so interactive content keeps working (unlike
    /// `GestureSingle`, which locks onto the first sequence it sees and
    /// cannot distribute touches across siblings).
    pub quad_touches: RefCell<Vec<TrackedTouch>>,
    /// Baseline for the active quad transform.
    pub quad_baseline: RefCell<Option<QuadBaseline>>,
    /// Sequences any gesture in the card subtree has claimed, each
    /// with the claim timestamp. Claims reset when a tap ends, so the
    /// state is not queryable retroactively - the mark keeps it: a
    /// claimed sequence's `TouchEnd` went to the claimer, never to
    /// the tracker, so its quad touch is a phantom. `EventSequence`
    /// is only a touch-slot id and is reused for the next press on
    /// the same slot, so marks expire after the stale window -
    /// afterwards the phantom would die on the stale check anyway.
    pub claimed: RefCell<HashMap<EventSequence, Instant>>,
    /// Gesture object addresses already carrying a
    /// `sequence-state-changed` watcher, so the lazy walk connects
    /// each controller only once.
    pub watched: RefCell<HashSet<usize>>,
    /// Insertion sequence number for stable z-ordering.
    pub sequence: SequenceNumber,
}

/// Smallest radial grab distance the resize drag accepts as its
/// reference - a press (nearly) on the card center would make tiny
/// pointer moves blow up the ratio.
const RESIZE_MIN_RADIUS: Pixels = Pixels::new(4.0);

impl CardEntry {
    /// Scale target for a resize-drag sample at pointer position `now`
    /// (board coordinates): the grab-time scale multiplied by the
    /// radial-distance ratio to the card center. `None` while no
    /// resize grab is active.
    pub(crate) fn resize_target(&self, now: Vec2<Pixels>) -> Option<ScaleFactor> {
        let grab = self.resize_grab.get()?;
        Some(resize_scale(grab, now, self.center(), self.resize_scale.get(), &self.config.scale_limits))
    }

    /// Current center position as a board-coordinate vector.
    fn center(&self) -> Vec2<Pixels> {
        Vec2::<Pixels>::from(self.position.get())
    }
}

/// Scale target for one resize-drag sample: `start` multiplied by the
/// ratio between the pointer's current and grab-time radial distance to
/// the card `center`, clamped to `limits`. Radial distance is
/// rotation-invariant, so a rotated card resizes identically to an
/// unrotated one. `start` is returned while the grab sits too close to
/// `center` to yield a stable ratio.
fn resize_scale(grab: Vec2<Pixels>, now: Vec2<Pixels>, center: Vec2<Pixels>, start: ScaleFactor, limits: &ScaleBounds) -> ScaleFactor {
    let grab_radius = grab.distance(&center);
    if grab_radius < RESIZE_MIN_RADIUS {
        return start;
    }
    // `Pixels / Pixels` yields the dimensionless same-unit ratio.
    let ratio = now.distance(&center) / grab_radius;
    ScaleFactor::new(start.value() * ratio).clamped(limits)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resize(grab: (f64, f64), now: (f64, f64), start: f64) -> f64 {
        resize_scale(
            Vec2::<Pixels>::from(grab),
            Vec2::<Pixels>::from(now),
            Vec2::<Pixels>::ZERO,
            ScaleFactor::new(start),
            &ScaleBounds::default(),
        )
        .value()
    }

    #[test]
    fn doubling_the_radial_distance_doubles_the_scale() {
        assert!((resize((100.0, 0.0), (200.0, 0.0), 1.0) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn halving_the_radial_distance_halves_the_scale() {
        assert!((resize((100.0, 0.0), (50.0, 0.0), 1.0) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn resize_ratio_is_rotation_invariant() {
        // Same radius, different angle: a rotated card scales identically.
        let radial = resize((100.0, 0.0), (200.0, 0.0), 1.0);
        let diagonal = resize((70.71067811865476, 70.71067811865476), (141.4213562373095, 141.4213562373095), 1.0);
        assert!((radial - diagonal).abs() < 1e-9);
    }

    #[test]
    fn resize_clamps_to_scale_bounds() {
        assert!((resize((100.0, 0.0), (1_000_000.0, 0.0), 1.0) - 2.0).abs() < f64::EPSILON);
        assert!((resize((100.0, 0.0), (0.000001, 0.0), 1.0) - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn grab_on_the_center_keeps_the_start_scale() {
        assert!((resize((1.0, 0.0), (500.0, 0.0), 1.5) - 1.5).abs() < f64::EPSILON);
    }
}

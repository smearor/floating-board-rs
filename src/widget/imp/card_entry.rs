use crate::domain::CardAnchor;
use crate::domain::CardGeometry;
use crate::domain::CardPosition;
use crate::domain::GenerationCounter;
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
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Vec2;
use smearor_commons_layout::Seat;
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
    /// Current center position in workspace coordinates.
    pub position: Cell<CardPosition>,
    /// Current rotation in workspace coordinates - the model value.
    /// The `RotationWidget` carries the rendered rotation
    /// (`rotation + view.rotation`); `notify::rotation` syncs this
    /// cell back from the widget.
    pub rotation: Cell<RotationRad>,
    /// Current scale factor (card scale; the rendered widget scale is
    /// `scale x view zoom`).
    pub scale: Cell<ScaleFactor>,
    /// Anchor awaiting resolution at the next board allocation.
    pub pending_anchor: RefCell<Option<CardAnchor>>,
    /// Card center at the start of the active drag gesture.
    pub drag_start: Cell<CardPosition>,
    /// Pointer position in workspace coordinates at drag begin.
    pub drag_grab: Cell<Option<Vec2<Pixels>>>,
    /// Pointer position in workspace coordinates at resize-drag begin;
    /// `Some` while a resize gesture on the card's grip is active.
    pub resize_grab: Cell<Option<Vec2<Pixels>>>,
    /// Scale factor captured at resize-drag begin; updates apply the
    /// ratio between the current and the grab radial distance.
    pub resize_scale: Cell<ScaleFactor>,
    /// Pointer position in workspace coordinates at rotate-drag begin;
    /// `Some` while a rotate gesture on the card's grip is active.
    pub rotate_grab: Cell<Option<Vec2<Pixels>>>,
    /// Whether the active rotate drag already consumed its quarter-turn
    /// step - the grip performs at most one step per drag, no matter
    /// how far the swipe continues past the trigger.
    pub rotate_stepped: Cell<bool>,
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
    /// Seat the card is assigned to on the tabletop edge -
    /// `Some` mirrors `config.seat` until a seat-changing operation
    /// (pass to next user) updates it.
    pub seat: Cell<Option<Seat>>,
    /// Pinned to the screen (`CardConfig::sticky`): the card bypasses
    /// the view camera - position, rotation and scale are widget-space
    /// while set.
    pub sticky: Cell<bool>,
    /// Generation counter of the running layout animation - a new
    /// `apply_layout` call bumps it and stale frame callbacks stop
    /// themselves instead of fighting the new animation.
    pub layout_generation: Cell<GenerationCounter>,
    /// Destination of the in-flight glide, tagged with the
    /// `layout_generation` that started it. Seat fan-out reads this as
    /// the card's effective geometry so passes during a glide fan out
    /// around the landing spot, not the mid-flight position. A
    /// superseded tag is ignored by readers; completion clears it.
    pub glide_target: Cell<Option<(GenerationCounter, CardGeometry)>>,
}

/// Smallest radial grab distance the resize drag accepts as its
/// reference - a press (nearly) on the card center would make tiny
/// pointer moves blow up the ratio.
const RESIZE_MIN_RADIUS: Pixels = Pixels::new(4.0);

/// Angular sweep around the card center that triggers the rotate
/// grip's quarter-turn step (5 degrees) - above pointer jitter, well
/// below the quarter turn itself.
const ROTATE_TRIGGER: RotationRad = RotationRad::new(5.0 * std::f64::consts::PI / 180.0);

/// Smallest pointer-to-center distance for a meaningful swipe
/// direction - a sample (nearly) on the card center has no stable
/// angle.
const ROTATE_MIN_RADIUS: Pixels = Pixels::new(8.0);

impl CardEntry {
    /// Scale target for a resize-drag sample at pointer position `now`
    /// (workspace coordinates): the grab-time scale multiplied by the
    /// radial-distance ratio to the card center. `None` while no
    /// resize grab is active.
    pub(crate) fn resize_target(&self, now: Vec2<Pixels>) -> Option<ScaleFactor> {
        let grab = self.resize_grab.get()?;
        Some(resize_scale(grab, now, self.center(), self.resize_scale.get(), &self.config.scale_limits))
    }

    /// Quarter-turn target for a rotate-drag sample at pointer position
    /// `now` (workspace coordinates): once the pointer's angular sweep
    /// around the card center passes `ROTATE_TRIGGER`, the next 90
    /// degree boundary in the sweep direction off `current`. `None`
    /// while no rotate grab is active, while the swipe is still below
    /// the trigger, or once the drag's single step was taken.
    pub(crate) fn rotate_target(&self, now: Vec2<Pixels>, current: RotationRad) -> Option<RotationRad> {
        if self.rotate_stepped.get() {
            return None;
        }
        let grab = self.rotate_grab.get()?;
        let center = self.center();
        if grab.distance(&center) < ROTATE_MIN_RADIUS || now.distance(&center) < ROTATE_MIN_RADIUS {
            return None;
        }
        let sweep = (grab - center).direction().signed_delta_to((now - center).direction());
        if sweep.abs() < ROTATE_TRIGGER {
            return None;
        }
        Some(next_quarter_turn(current, sweep > RotationRad::ZERO))
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

/// The quarter-turn boundary next in `direction` from `current`: the
/// smallest 90 degree multiple past `current` when `clockwise`, the
/// largest below it when not. A `current` already on a boundary steps
/// a full quarter turn; the epsilon keeps float noise on snapped
/// angles from staying put.
fn next_quarter_turn(current: RotationRad, clockwise: bool) -> RotationRad {
    const QUARTER: f64 = 90.0;
    const EPSILON: f64 = 1e-6;
    let turns = current.to_degrees() / QUARTER;
    let next = if clockwise {
        (turns + EPSILON).floor() + 1.0
    } else {
        (turns - EPSILON).ceil() - 1.0
    };
    RotationRad::from_degrees(next * QUARTER)
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

    fn next(current: f64, clockwise: bool) -> f64 {
        next_quarter_turn(RotationRad::from_degrees(current), clockwise).to_degrees()
    }

    #[test]
    fn snapped_rotation_steps_a_full_quarter_turn() {
        assert_eq!(next(0.0, true), 90.0);
        assert_eq!(next(90.0, true), 180.0);
        assert_eq!(next(90.0, false), 0.0);
        assert_eq!(next(0.0, false), -90.0);
    }

    #[test]
    fn free_rotation_lands_on_the_next_boundary() {
        assert_eq!(next(30.0, true), 90.0);
        assert_eq!(next(30.0, false), 0.0);
        assert_eq!(next(-10.0, false), -90.0);
        assert_eq!(next(269.0, true), 270.0);
    }

    #[test]
    fn float_noise_on_a_boundary_still_steps() {
        assert_eq!(next(90.0 + 1e-9, true), 180.0);
        assert_eq!(next(90.0 - 1e-9, false), 0.0);
    }
}

use crate::domain::CardAnchor;
use crate::domain::CardPosition;
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

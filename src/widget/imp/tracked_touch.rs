use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::time::Instant;

/// One finger tracked by the quad transform: the touch sequence, its latest
/// board-space point and when the last event for it arrived (the timestamp
/// detects touches whose `TouchEnd` was routed to a claiming gesture and
/// never reached the tracker).
#[derive(Debug, Clone)]
pub(crate) struct TrackedTouch {
    /// The touch sequence identifying this finger.
    pub sequence: gtk4::gdk::EventSequence,
    /// Latest finger position in board coordinates.
    pub point: Vec2<Pixels>,
    /// Timestamp of the most recent event for this sequence.
    pub last_event: Instant,
}

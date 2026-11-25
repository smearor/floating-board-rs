//! Runtime state of a playing fold transition on a card entry - the
//! board's tick evaluates `animation`'s eased progress through
//! `CardFoldAnimation::pose` and applies the pose per frame.

use crate::domain::CardFoldAnimation;
use smearor_commons_animation::Span;
use smearor_commons_animation::TimedAnimation;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::ScaleFactor;
use smearor_commons_geometry::Vec2;

/// One in-flight fold transition: eased progress drives `kind`'s pose
/// between `start_size` and `end_size`; rotation/zoom excursions
/// compose on the captured rendered bases so user gestures before
/// the fold are preserved.
pub struct FoldAnimation {
    /// Eased progress driver (`0.0` = start state, `1.0` = end state).
    pub animation: TimedAnimation<Span<f64>>,
    /// `true` = animating into the folded state.
    pub to_folded: bool,
    /// The direction-resolved morph program
    /// (`CardConfig::effective_fold_animation`).
    pub kind: CardFoldAnimation,
    /// Eased-progress fraction at which a gated satellite reveal
    /// fires (unfold direction only): `unfold_timing`'s `Overlap`
    /// fraction, `Fraction::ONE` for `Sequential`.
    pub reveal_at: Fraction,
    /// Card footprint in px at the transition start.
    pub start_size: Vec2<Pixels>,
    /// Card footprint in px at the transition end.
    pub end_size: Vec2<Pixels>,
    /// Rendered base rotation the excursion composes on.
    pub base_rotation: RotationRad,
    /// Rendered base scale the zoom excursion composes on.
    pub base_scale: ScaleFactor,
}

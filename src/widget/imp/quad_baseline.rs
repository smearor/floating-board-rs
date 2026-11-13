use crate::domain::CardPosition;
use crate::domain::ScaleFactor;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Vec2;

/// Snapshot of the card geometry and quad points taken when a quad
/// transform (re-)activates; refreshed whenever the point count changes so
/// adding or lifting fingers never causes a jump.
pub(crate) struct QuadBaseline {
    /// Card center when the baseline was taken (board coordinates).
    pub position: CardPosition,
    /// Scale factor at baseline.
    pub scale: ScaleFactor,
    /// Rotation at baseline.
    pub rotation: RotationRad,
    /// Centroid of `points` (board coordinates).
    pub centroid: Vec2<Pixels>,
    /// Board-space point per active quad slot at baseline.
    pub points: Vec<Vec2<Pixels>>,
    /// Whether the transform already changed the position.
    pub moved: bool,
    /// Whether the transform already changed the rotation.
    pub rotated: bool,
    /// Whether the transform already changed the scale.
    pub scaled: bool,
    /// The angle applied by the most recent transform update, used to
    /// detect degenerate finger crossings (~180° flips between frames).
    pub applied_angle: RotationRad,
}

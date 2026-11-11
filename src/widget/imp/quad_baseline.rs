use crate::domain::BoardPoint;
use crate::domain::CardPosition;
use crate::domain::RotationRad;

/// Snapshot of the card geometry and quad points taken when a quad
/// transform (re-)activates; refreshed whenever the point count changes so
/// adding or lifting fingers never causes a jump.
pub(crate) struct QuadBaseline {
    /// Card center when the baseline was taken (board coordinates).
    pub position: CardPosition,
    /// Scale factor at baseline.
    pub scale: f64,
    /// Rotation at baseline.
    pub rotation: RotationRad,
    /// Centroid of `points` (board coordinates).
    pub centroid: BoardPoint,
    /// Board-space point per active quad slot at baseline.
    pub points: Vec<BoardPoint>,
    /// Whether the transform already changed the position.
    pub moved: bool,
    /// Whether the transform already changed the rotation.
    pub rotated: bool,
    /// Whether the transform already changed the scale.
    pub scaled: bool,
    /// The angle applied by the most recent transform update, used to
    /// detect degenerate finger crossings (~180° flips between frames).
    pub applied_angle_deg: f64,
}

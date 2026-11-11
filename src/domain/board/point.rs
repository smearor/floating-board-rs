/// A point in board coordinates (pixels), used wherever geometry is
/// captured - touch positions, point-cloud centroids, drag grab points.
///
/// Unlike [`crate::domain::CardPosition`] it carries no card-anchor
/// semantics; it only marks raw `(x, y)` tuples as board-space so they
/// cannot be confused with widget-space points or plain delta vectors.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoardPoint(pub f64, pub f64);

impl BoardPoint {
    /// Creates a point from board coordinates.
    pub fn new(x: f64, y: f64) -> Self {
        Self(x, y)
    }

    /// Horizontal board coordinate.
    pub fn x(&self) -> f64 {
        self.0
    }

    /// Vertical board coordinate.
    pub fn y(&self) -> f64 {
        self.1
    }

    /// Centroid of a point set; the empty set yields the origin.
    pub fn centroid(points: &[BoardPoint]) -> Self {
        let count = points.len().max(1) as f64;
        Self(points.iter().map(|p| p.x()).sum::<f64>() / count, points.iter().map(|p| p.y()).sum::<f64>() / count)
    }

    /// Returns the point rotated by `degrees` (clockwise, screen
    /// coordinates).
    pub fn rotated(&self, degrees: f64) -> Self {
        let radians = degrees.to_radians();
        let (sin, cos) = radians.sin_cos();
        Self(self.0 * cos - self.1 * sin, self.0 * sin + self.1 * cos)
    }
}

impl From<(f64, f64)> for BoardPoint {
    fn from(value: (f64, f64)) -> Self {
        Self(value.0, value.1)
    }
}

impl From<BoardPoint> for (f64, f64) {
    fn from(value: BoardPoint) -> Self {
        (value.0, value.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centroid_of_quad_points() {
        let points = vec![
            BoardPoint::new(0.0, 0.0),
            BoardPoint::new(10.0, 0.0),
            BoardPoint::new(10.0, 10.0),
            BoardPoint::new(0.0, 10.0),
        ];
        assert_eq!(BoardPoint::centroid(&points), BoardPoint::new(5.0, 5.0));
    }

    #[test]
    fn centroid_of_empty_set_is_origin() {
        assert_eq!(BoardPoint::centroid(&[]), BoardPoint::new(0.0, 0.0));
    }

    const EPSILON: f64 = 1e-9;

    #[test]
    fn rotated_quarter_turns() {
        let point = BoardPoint::new(1.0, 0.0).rotated(90.0);
        assert!((point.x() - 0.0).abs() < EPSILON && (point.y() - 1.0).abs() < EPSILON);
        let point = BoardPoint::new(1.0, 0.0).rotated(180.0);
        assert!((point.x() + 1.0).abs() < EPSILON && point.y().abs() < EPSILON);
        let point = BoardPoint::new(0.0, 1.0).rotated(270.0);
        assert!((point.x() - 1.0).abs() < EPSILON && point.y().abs() < EPSILON);
    }
}

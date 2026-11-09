use crate::domain::CardPosition;

/// Axis-aligned rectangle in board coordinates (pixels).
///
/// Used for card footprints and board bounds in the pure geometry math
/// (`anchor`, `clamp`) - deliberately independent of `graphene::Rect` so the
/// domain stays GTK-free.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Rect {
    /// Left edge x-coordinate.
    pub x: f64,
    /// Top edge y-coordinate.
    pub y: f64,
    /// Width in pixels.
    pub width: f64,
    /// Height in pixels.
    pub height: f64,
}

impl Rect {
    /// Creates a rectangle from top-left corner and size.
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    /// Creates the rectangle of the given size centered at `center`.
    pub fn centered(center: CardPosition, width: f64, height: f64) -> Self {
        Self {
            x: center.x() - width / 2.0,
            y: center.y() - height / 2.0,
            width,
            height,
        }
    }

    /// Right edge x-coordinate.
    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    /// Bottom edge y-coordinate.
    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    /// Center point of the rectangle.
    pub fn center(&self) -> CardPosition {
        CardPosition::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Returns whether the rectangle has a positive area.
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Returns the rectangle translated by the given offset.
    pub fn translated(&self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..*self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_edges_and_center() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(rect.right(), 110.0);
        assert_eq!(rect.bottom(), 70.0);
        assert_eq!(rect.center(), CardPosition::new(60.0, 45.0));
    }

    #[test]
    fn rect_centered_roundtrip() {
        let center = CardPosition::new(200.0, 100.0);
        let rect = Rect::centered(center, 80.0, 40.0);
        assert_eq!(rect.center(), center);
        assert_eq!(rect, Rect::new(160.0, 80.0, 80.0, 40.0));
    }
}

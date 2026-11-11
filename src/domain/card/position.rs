/// Position of a card's **center point** in board coordinates (pixels).
///
/// The center is used instead of the top-left corner because it stays stable
/// under rotation and scaling: rotating or scaling a card never moves its
/// center anchor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CardPosition(pub f64, pub f64);

impl CardPosition {
    /// Creates a position from board coordinates.
    pub fn new(x: f64, y: f64) -> Self {
        Self(x, y)
    }

    /// Horizontal board coordinate of the card center.
    pub fn x(&self) -> f64 {
        self.0
    }

    /// Vertical board coordinate of the card center.
    pub fn y(&self) -> f64 {
        self.1
    }

    /// Returns the position translated by the given offset.
    pub fn translated(&self, dx: f64, dy: f64) -> Self {
        Self(self.0 + dx, self.1 + dy)
    }
}

impl From<(f64, f64)> for CardPosition {
    fn from(value: (f64, f64)) -> Self {
        Self(value.0, value.1)
    }
}

impl From<CardPosition> for (f64, f64) {
    fn from(value: CardPosition) -> Self {
        (value.0, value.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_accessors() {
        let position = CardPosition::new(10.0, 20.0);
        assert_eq!(position.x(), 10.0);
        assert_eq!(position.y(), 20.0);
    }

    #[test]
    fn position_translation() {
        let position = CardPosition::new(10.0, 20.0).translated(5.0, -5.0);
        assert_eq!(position, CardPosition::new(15.0, 15.0));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn position_serializes_as_pair() {
        let json = serde_json::to_string(&CardPosition::new(480.0, 660.0)).unwrap();
        assert_eq!(json, "[480.0,660.0]");
        let parsed: CardPosition = serde_json::from_str("[1.5,2.5]").unwrap();
        assert_eq!(parsed, CardPosition::new(1.5, 2.5));
    }
}

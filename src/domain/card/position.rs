use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;

/// Position of a card's **center point** in board coordinates (pixels).
///
/// The center is used instead of the top-left corner because it stays stable
/// under rotation and scaling: rotating or scaling a card never moves its
/// center anchor. Wraps [`Vec2<Pixels>`] with card-domain semantics; serde
/// keeps the flat `[x, y]` pair shape.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CardPosition(
    /// Center point in board coordinates.
    #[cfg_attr(feature = "serde", serde(with = "smearor_commons_geometry::serde_pair"))]
    pub Vec2<Pixels>,
);

impl CardPosition {
    /// Creates a position from a board coordinate.
    pub fn new(position: Vec2<Pixels>) -> Self {
        Self(position)
    }

    /// Horizontal board coordinate of the card center.
    pub fn x(&self) -> Pixels {
        self.0.x_unit()
    }

    /// Vertical board coordinate of the card center.
    pub fn y(&self) -> Pixels {
        self.0.y_unit()
    }

    /// Returns the position translated by the given offset.
    pub fn translated(&self, delta: Vec2<Pixels>) -> Self {
        Self(self.0 + delta)
    }
}

impl From<(f64, f64)> for CardPosition {
    fn from(value: (f64, f64)) -> Self {
        Self(Vec2::new(value.0, value.1))
    }
}

impl From<CardPosition> for (f64, f64) {
    fn from(value: CardPosition) -> Self {
        (value.x().as_f64(), value.y().as_f64())
    }
}

impl From<Vec2<Pixels>> for CardPosition {
    fn from(value: Vec2<Pixels>) -> Self {
        Self(value)
    }
}

impl From<CardPosition> for Vec2<Pixels> {
    fn from(value: CardPosition) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_accessors() {
        let position = CardPosition::from(Vec2::new(10.0, 20.0));
        assert_eq!(position.x().as_f64(), 10.0);
        assert_eq!(position.y().as_f64(), 20.0);
    }

    #[test]
    fn position_translation() {
        let position = CardPosition::from(Vec2::new(10.0, 20.0)).translated(Vec2::new(5.0, -5.0));
        assert_eq!(position, CardPosition::from(Vec2::new(15.0, 15.0)));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn position_serializes_as_pair() {
        let json = serde_json::to_string(&CardPosition::from(Vec2::new(480.0, 660.0))).unwrap();
        assert_eq!(json, "[480.0,660.0]");
        let parsed: CardPosition = serde_json::from_str("[1.5,2.5]").unwrap();
        assert_eq!(parsed, CardPosition::from(Vec2::new(1.5, 2.5)));
    }
}

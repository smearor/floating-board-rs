use crate::domain::CardPosition;

/// Declarative initial placement of a card on the board.
///
/// Anchors are resolved against the board bounds once the card's natural size
/// is known (first allocation). [`CardAnchor::Explicit`] bypasses resolution
/// and uses the given center position directly.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum CardAnchor {
    /// Top edge, horizontally flush left.
    TopLeft,
    /// Top edge, horizontally centered.
    TopCenter,
    /// Top edge, horizontally flush right.
    TopRight,
    /// Left edge, vertically centered.
    CenterLeft,
    /// Board center.
    #[default]
    Center,
    /// Right edge, vertically centered.
    CenterRight,
    /// Bottom edge, horizontally flush left.
    BottomLeft,
    /// Bottom edge, horizontally centered.
    BottomCenter,
    /// Bottom edge, horizontally flush right.
    BottomRight,
    /// Explicit center position in board coordinates.
    Explicit(CardPosition),
}

impl CardAnchor {
    /// Horizontal anchor weight: `0.0` = left, `0.5` = center, `1.0` = right.
    pub(crate) fn horizontal_weight(&self) -> Option<f64> {
        match self {
            Self::TopLeft | Self::CenterLeft | Self::BottomLeft => Some(0.0),
            Self::TopCenter | Self::Center | Self::BottomCenter => Some(0.5),
            Self::TopRight | Self::CenterRight | Self::BottomRight => Some(1.0),
            Self::Explicit(_) => None,
        }
    }

    /// Vertical anchor weight: `0.0` = top, `0.5` = center, `1.0` = bottom.
    pub(crate) fn vertical_weight(&self) -> Option<f64> {
        match self {
            Self::TopLeft | Self::TopCenter | Self::TopRight => Some(0.0),
            Self::CenterLeft | Self::Center | Self::CenterRight => Some(0.5),
            Self::BottomLeft | Self::BottomCenter | Self::BottomRight => Some(1.0),
            Self::Explicit(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_weights() {
        assert_eq!(CardAnchor::BottomRight.horizontal_weight(), Some(1.0));
        assert_eq!(CardAnchor::BottomRight.vertical_weight(), Some(1.0));
        assert_eq!(CardAnchor::Center.horizontal_weight(), Some(0.5));
        assert_eq!(CardAnchor::Explicit(CardPosition::new(1.0, 2.0)).horizontal_weight(), None);
    }
}

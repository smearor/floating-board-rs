use crate::domain::CardPosition;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;

/// Default margin in pixels between a card's footprint and the board edge
/// when an edge/corner anchor is used. Cards override it via
/// `CardConfig::anchor_margin`.
pub const ANCHOR_MARGIN: f64 = 24.0;

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

    /// Resolves the anchor into a concrete center position.
    ///
    /// `bounds` is the board rectangle, `footprint` the card's displayed
    /// size (already accounting for rotation and scale). Edge anchors place
    /// the footprint `margin` pixels inside the bounds;
    /// [`CardAnchor::Explicit`] returns its position unchanged.
    pub fn resolve(&self, bounds: &Rect<Pixels>, footprint_width: f64, footprint_height: f64, margin: f64) -> CardPosition {
        if let Self::Explicit(position) = self {
            return *position;
        }
        let horizontal = self.horizontal_weight().unwrap_or(0.5);
        let vertical = self.vertical_weight().unwrap_or(0.5);
        let inset_x = margin + footprint_width / 2.0;
        let inset_y = margin + footprint_height / 2.0;
        let x = bounds.min.x + inset_x + (bounds.size.x - 2.0 * inset_x).max(0.0) * horizontal;
        let y = bounds.min.y + inset_y + (bounds.size.y - 2.0 * inset_y).max(0.0) * vertical;
        CardPosition::new(x, y)
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

    const BOUNDS: Rect<Pixels> = Rect::from_xywh(0.0, 0.0, 1000.0, 800.0);

    #[test]
    fn center_anchor_centers_card() {
        let position = CardAnchor::Center.resolve(&BOUNDS, 200.0, 100.0, ANCHOR_MARGIN);
        assert_eq!(position, CardPosition::new(500.0, 400.0));
    }

    #[test]
    fn bottom_center_anchor_insets_from_bottom() {
        let position = CardAnchor::BottomCenter.resolve(&BOUNDS, 200.0, 100.0, ANCHOR_MARGIN);
        assert_eq!(position, CardPosition::new(500.0, 800.0 - ANCHOR_MARGIN - 50.0));
    }

    #[test]
    fn top_left_anchor_insets_both_axes() {
        let position = CardAnchor::TopLeft.resolve(&BOUNDS, 200.0, 100.0, ANCHOR_MARGIN);
        assert_eq!(position, CardPosition::new(ANCHOR_MARGIN + 100.0, ANCHOR_MARGIN + 50.0));
    }

    #[test]
    fn explicit_anchor_passthrough() {
        let explicit = CardPosition::new(123.0, 456.0);
        assert_eq!(CardAnchor::Explicit(explicit).resolve(&BOUNDS, 200.0, 100.0, ANCHOR_MARGIN), explicit);
    }

    #[test]
    fn tiny_board_stays_inside() {
        let bounds = Rect::<Pixels>::from_xywh(0.0, 0.0, 100.0, 80.0);
        let position = CardAnchor::BottomRight.resolve(&bounds, 400.0, 300.0, ANCHOR_MARGIN);
        assert_eq!(position, CardPosition::new(ANCHOR_MARGIN + 200.0, ANCHOR_MARGIN + 150.0));
    }
}

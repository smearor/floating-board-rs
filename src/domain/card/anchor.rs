use crate::domain::CardPosition;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Vec2;

/// Default margin in pixels between a card's footprint and the board edge
/// when an edge/corner anchor is used. Cards override it via
/// `CardConfig::anchor_margin`.
pub const ANCHOR_MARGIN: Pixels = Pixels::new(24.0);

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
    pub fn resolve(&self, bounds: &Rect<Pixels>, footprint: Vec2<Pixels>, margin: Pixels) -> CardPosition {
        if let Self::Explicit(position) = self {
            return *position;
        }
        let weights = Vec2::new(self.horizontal_weight().unwrap_or(0.5), self.vertical_weight().unwrap_or(0.5));
        let inset = footprint * 0.5 + Vec2::splat(margin);
        let span = (bounds.size - inset * 2.0).map(|c| c.max(0.0));
        CardPosition::from(bounds.min + inset + span.component_mul(&weights))
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
        assert_eq!(CardAnchor::Explicit(CardPosition::from(Vec2::new(1.0, 2.0))).horizontal_weight(), None);
    }

    const BOUNDS: Rect<Pixels> = Rect::from_xywh(0.0, 0.0, 1000.0, 800.0);

    #[test]
    fn center_anchor_centers_card() {
        let position = CardAnchor::Center.resolve(&BOUNDS, Vec2::new(200.0, 100.0), ANCHOR_MARGIN);
        assert_eq!(position, CardPosition::from(Vec2::new(500.0, 400.0)));
    }

    #[test]
    fn bottom_center_anchor_insets_from_bottom() {
        let position = CardAnchor::BottomCenter.resolve(&BOUNDS, Vec2::new(200.0, 100.0), ANCHOR_MARGIN);
        assert_eq!(position, CardPosition::from(Vec2::new(500.0, 800.0 - ANCHOR_MARGIN.as_f64() - 50.0)));
    }

    #[test]
    fn top_left_anchor_insets_both_axes() {
        let position = CardAnchor::TopLeft.resolve(&BOUNDS, Vec2::new(200.0, 100.0), ANCHOR_MARGIN);
        assert_eq!(position, CardPosition::from(Vec2::new(ANCHOR_MARGIN.as_f64() + 100.0, ANCHOR_MARGIN.as_f64() + 50.0)));
    }

    #[test]
    fn explicit_anchor_passthrough() {
        let explicit = CardPosition::from(Vec2::new(123.0, 456.0));
        assert_eq!(CardAnchor::Explicit(explicit).resolve(&BOUNDS, Vec2::new(200.0, 100.0), ANCHOR_MARGIN), explicit);
    }

    #[test]
    fn tiny_board_stays_inside() {
        let bounds = Rect::<Pixels>::from_xywh(0.0, 0.0, 100.0, 80.0);
        let position = CardAnchor::BottomRight.resolve(&bounds, Vec2::new(400.0, 300.0), ANCHOR_MARGIN);
        assert_eq!(position, CardPosition::from(Vec2::new(ANCHOR_MARGIN.as_f64() + 200.0, ANCHOR_MARGIN.as_f64() + 150.0)));
    }
}

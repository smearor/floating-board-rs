use crate::domain::CardAnchor;
use crate::domain::CardPosition;
use crate::domain::Rect;

/// Margin in pixels between a card's footprint and the board edge when an
/// edge/corner anchor is used.
pub const ANCHOR_MARGIN: f64 = 24.0;

/// Resolves a [`CardAnchor`] into a concrete center position.
///
/// `bounds` is the board rectangle, `footprint` the card's displayed size
/// (already accounting for rotation and scale). Edge anchors place the
/// footprint `ANCHOR_MARGIN` inside the bounds; [`CardAnchor::Explicit`]
/// returns its position unchanged.
pub fn resolve_anchor(anchor: &CardAnchor, bounds: &Rect, footprint_width: f64, footprint_height: f64) -> CardPosition {
    if let CardAnchor::Explicit(position) = anchor {
        return *position;
    }
    let horizontal = anchor.horizontal_weight().unwrap_or(0.5);
    let vertical = anchor.vertical_weight().unwrap_or(0.5);
    let inset_x = ANCHOR_MARGIN + footprint_width / 2.0;
    let inset_y = ANCHOR_MARGIN + footprint_height / 2.0;
    let x = bounds.x + inset_x + (bounds.width - 2.0 * inset_x).max(0.0) * horizontal;
    let y = bounds.y + inset_y + (bounds.height - 2.0 * inset_y).max(0.0) * vertical;
    CardPosition::new(x, y)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 1000.0,
        height: 800.0,
    };

    #[test]
    fn center_anchor_centers_card() {
        let position = resolve_anchor(&CardAnchor::Center, &BOUNDS, 200.0, 100.0);
        assert_eq!(position, CardPosition::new(500.0, 400.0));
    }

    #[test]
    fn bottom_center_anchor_insets_from_bottom() {
        let position = resolve_anchor(&CardAnchor::BottomCenter, &BOUNDS, 200.0, 100.0);
        assert_eq!(position, CardPosition::new(500.0, 800.0 - ANCHOR_MARGIN - 50.0));
    }

    #[test]
    fn top_left_anchor_insets_both_axes() {
        let position = resolve_anchor(&CardAnchor::TopLeft, &BOUNDS, 200.0, 100.0);
        assert_eq!(position, CardPosition::new(ANCHOR_MARGIN + 100.0, ANCHOR_MARGIN + 50.0));
    }

    #[test]
    fn explicit_anchor_passthrough() {
        let explicit = CardPosition::new(123.0, 456.0);
        assert_eq!(resolve_anchor(&CardAnchor::Explicit(explicit), &BOUNDS, 200.0, 100.0), explicit);
    }

    #[test]
    fn tiny_board_stays_inside() {
        let bounds = Rect::new(0.0, 0.0, 100.0, 80.0);
        let position = resolve_anchor(&CardAnchor::BottomRight, &bounds, 400.0, 300.0);
        assert_eq!(position, CardPosition::new(ANCHOR_MARGIN + 200.0, ANCHOR_MARGIN + 150.0));
    }
}

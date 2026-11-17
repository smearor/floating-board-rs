use crate::domain::CardId;
use crate::domain::CardPosition;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Vec2;

/// A card's alignment state - where the board model says the card is,
/// where the card layer stored the transform stack, and where the
/// card actually renders. Diagnostics API for input-region coverage
/// checks: the alignment invariant is `rendered.center() == position`
/// (rotation and scale apply around the center, so the rendered AABB
/// stays centered), and [`drift`](Self::drift) quantifies the
/// violation.
#[derive(Debug, Clone)]
pub struct CardAlignment {
    /// The card's id.
    pub card_id: CardId,
    /// Model center position in workspace coordinates.
    pub position: CardPosition,
    /// Stored `put` spot of the transform stack on the layer - `None`
    /// while the widget tree is not attached yet.
    pub placed: Option<Vec2<Pixels>>,
    /// Rendered bounds in widget coordinates - `None` while the card
    /// is not on a board or not realized yet.
    pub rendered: Option<Rect<Pixels>>,
    /// Rendered-center minus model position - the alignment
    /// invariant's violation; `None` while the card does not render.
    pub drift: Option<Vec2<Pixels>>,
}

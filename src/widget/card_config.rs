use crate::domain::CardAnchor;
use crate::domain::DragHandle;
use crate::domain::MinVisible;
use crate::domain::ScaleBounds;

/// Placement and interaction configuration of a card.
///
/// Passed to `FloatingBoard::add_card`; all fields have defaults so
/// `CardConfig::default()` yields a fully interactive, draggable card anchored
/// at the board center.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CardConfig {
    /// Title shown in the header strip; falls back to a localized default when
    /// the header is visible and no title is set.
    pub title: Option<String>,
    /// Initial placement anchor, resolved on first allocation.
    pub position: CardAnchor,
    /// Whether the card can be moved by dragging.
    pub draggable: bool,
    /// Whether the card can be rotated via the two-finger rotate gesture.
    pub rotatable: bool,
    /// Whether the card can be scaled via the pinch gesture.
    pub scalable: bool,
    /// Whether the card can be folded down to its header.
    pub foldable: bool,
    /// Whether the header shows a close button that removes the card.
    pub closable: bool,
    /// Collapses the header to a slim grip when no title/buttons are needed.
    pub headerless: bool,
    /// Which part of the card acts as the drag affordance.
    pub drag_handle: DragHandle,
    /// Inclusive scale limits for the pinch gesture; default `0.5 - 2.0`.
    pub scale_limits: ScaleBounds,
    /// Minimum overlap in pixels a card must keep inside the board bounds.
    pub min_visible: MinVisible,
    /// Optional initial z-ordering; `None` appends the card on top.
    pub z_index: Option<i32>,
}

impl Default for CardConfig {
    fn default() -> Self {
        Self {
            title: None,
            position: CardAnchor::Center,
            draggable: true,
            rotatable: true,
            scalable: true,
            foldable: false,
            closable: false,
            headerless: false,
            drag_handle: DragHandle::Header,
            scale_limits: ScaleBounds::default(),
            min_visible: MinVisible::default(),
            z_index: None,
        }
    }
}

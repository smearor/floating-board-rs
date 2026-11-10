use crate::domain::ANCHOR_MARGIN;
use crate::domain::BoundsMode;
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
    /// Margin in pixels an edge or corner anchor keeps between the card's
    /// footprint and the board edge; default `ANCHOR_MARGIN` (24 px).
    /// Ignored for `Center` and `Explicit` placements.
    #[cfg_attr(feature = "serde", serde(default = "default_anchor_margin"))]
    pub anchor_margin: f64,
    /// Inclusive scale limits for the pinch gesture; default `0.5 - 2.0`.
    pub scale_limits: ScaleBounds,
    /// Minimum overlap in pixels a card must keep inside the board bounds.
    pub min_visible: MinVisible,
    /// How strictly the card's footprint is kept inside the board bounds;
    /// default `Loose` (only `min_visible` is enforced).
    #[cfg_attr(feature = "serde", serde(default))]
    pub bounds_mode: BoundsMode,
    /// Optional initial z-ordering; `None` appends the card on top.
    pub z_index: Option<i32>,
    /// Hard upper bound on the card's measured width in pixels; `None` lets
    /// the card grow with its content. Caps growth caused by changing content
    /// (e.g. a number gaining a digit): the layout allocates at most this
    /// size and overflowing content is clipped by the default CSS.
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_width: Option<i32>,
    /// Hard upper bound on the card's measured height in pixels; see
    /// `max_width`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub max_height: Option<i32>,
    /// Duration of rotation animations (gesture snapping and
    /// `set_rotation_with_animation`) in milliseconds; default 500.
    #[cfg_attr(feature = "serde", serde(default = "default_animation_speed_ms"))]
    pub animation_speed_ms: u64,
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
            anchor_margin: ANCHOR_MARGIN,
            scale_limits: ScaleBounds::default(),
            min_visible: MinVisible::default(),
            bounds_mode: BoundsMode::default(),
            z_index: None,
            max_width: None,
            max_height: None,
            animation_speed_ms: 500,
        }
    }
}

#[cfg(feature = "serde")]
fn default_anchor_margin() -> f64 {
    ANCHOR_MARGIN
}

#[cfg(feature = "serde")]
fn default_animation_speed_ms() -> u64 {
    500
}

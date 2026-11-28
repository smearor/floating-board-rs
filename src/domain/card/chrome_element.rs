/// One modular chrome element of a card - a control the card chrome is
/// composed of, independent of where it is mounted.
///
/// The element says *what* renders, not *where*: [`crate::domain::ChromePlacement`]
/// decides whether an element docks inline into the card or is mounted
/// as a satellite next to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum ChromeElement {
    /// The drag affordance shown as a grip icon inside the header
    /// (`CardConfig::drag_handle == DragHandle::Handle`).
    DragHandle,
    /// The card title label.
    Title,
    /// The fold button collapsing the card to its header
    /// (`CardConfig::foldable`).
    Minimize,
    /// The close button performing `CardConfig::close_action`
    /// (`CardConfig::closable`).
    Close,
    /// The button stepping the card one snap step clockwise
    /// (`Card::rotate_to_next_snap`) - the precise, tap-friendly
    /// counterpart of [`ChromeElement::RotateGrip`].
    RotateClockwise,
    /// The button stepping the card one snap step counter-clockwise
    /// (`Card::rotate_to_next_snap`).
    RotateCounterClockwise,
    /// The button passing the card one table side clockwise
    /// (`Card::pass_to_next_user` with `TurnDirection::Clockwise`) -
    /// the geometric next-seat step, not roster-aware.
    NextSeat,
    /// The button passing the card one table side counter-clockwise
    /// (`Card::pass_to_next_user` with
    /// `TurnDirection::CounterClockwise`).
    PreviousSeat,
    /// The bottom-right resize affordance (`CardConfig::resizable`).
    ResizeGrip,
    /// The bottom-left rotate affordance (`CardConfig::rotate_grip`).
    RotateGrip,
}

impl ChromeElement {
    /// Whether the element may be mounted as a satellite.
    ///
    /// `Minimize`, `Close` and the rotate/pass steps are actions and
    /// translate naturally to satellite buttons. `DragHandle` and
    /// `Title` are place-bound chrome (an affordance away from the
    /// card cannot drag or label it sensibly), and the grips are
    /// drag *areas* - the rotate steps are exactly the "grips return
    /// as actions" variant noted in the concept.
    pub fn supports_satellite(&self) -> bool {
        matches!(
            self,
            Self::Minimize | Self::Close | Self::RotateClockwise | Self::RotateCounterClockwise | Self::NextSeat | Self::PreviousSeat
        )
    }

    /// Whether the element can mount in a circle card's top-arc
    /// chrome band: every element except the edge grips - they need a
    /// straight card edge to drag on.
    pub fn supports_arc(&self) -> bool {
        !matches!(self, Self::ResizeGrip | Self::RotateGrip)
    }

    /// The element's deterministic id suffix for chrome satellites
    /// (`chrome-minimize`) - `None` for elements without satellite
    /// support, matching [`Self::supports_satellite`].
    pub fn satellite_key(&self) -> Option<&'static str> {
        match self {
            Self::Minimize => Some("minimize"),
            Self::Close => Some("close"),
            Self::RotateClockwise => Some("rotate-clockwise"),
            Self::RotateCounterClockwise => Some("rotate-counter-clockwise"),
            Self::NextSeat => Some("next-seat"),
            Self::PreviousSeat => Some("previous-seat"),
            Self::DragHandle | Self::Title | Self::ResizeGrip | Self::RotateGrip => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn satellite_support_matrix() {
        assert!(!ChromeElement::DragHandle.supports_satellite());
        assert!(!ChromeElement::Title.supports_satellite());
        assert!(ChromeElement::Minimize.supports_satellite());
        assert!(ChromeElement::Close.supports_satellite());
        assert!(ChromeElement::RotateClockwise.supports_satellite());
        assert!(ChromeElement::RotateCounterClockwise.supports_satellite());
        assert!(ChromeElement::NextSeat.supports_satellite());
        assert!(ChromeElement::PreviousSeat.supports_satellite());
        assert!(!ChromeElement::ResizeGrip.supports_satellite());
        assert!(!ChromeElement::RotateGrip.supports_satellite());
        assert!(ChromeElement::Title.supports_arc());
        assert!(ChromeElement::DragHandle.supports_arc());
        assert!(ChromeElement::Minimize.supports_arc());
        assert!(!ChromeElement::ResizeGrip.supports_arc());
        assert!(!ChromeElement::RotateGrip.supports_arc());
    }
}

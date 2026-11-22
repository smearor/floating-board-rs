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
    /// The bottom-right resize affordance (`CardConfig::resizable`).
    ResizeGrip,
    /// The bottom-left rotate affordance (`CardConfig::rotate_grip`).
    RotateGrip,
}

impl ChromeElement {
    /// Whether the element may be mounted as a satellite.
    ///
    /// `Minimize` and `Close` are actions and translate naturally to
    /// satellite buttons. `DragHandle` and `Title` are place-bound
    /// chrome (an affordance away from the card cannot drag or label
    /// it sensibly), and the grips are drag *areas* - they may return
    /// later as resize/rotate *actions*, not as satellites of their
    /// current form.
    pub fn supports_satellite(&self) -> bool {
        matches!(self, Self::Minimize | Self::Close)
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
        assert!(!ChromeElement::ResizeGrip.supports_satellite());
        assert!(!ChromeElement::RotateGrip.supports_satellite());
    }
}

/// Which part of a card acts as the drag affordance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum DragHandle {
    /// Only the header strip moves the card; interactive content stays usable.
    #[default]
    Header,
    /// The whole card surface is the drag affordance (display-only cards).
    WholeCard,
    /// A dedicated handle widget inside the header (grip icon).
    Handle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_handle_default_is_header() {
        assert_eq!(DragHandle::default(), DragHandle::Header);
    }
}

use crate::domain::ChromeElement;
use crate::domain::ChromePlacement;

/// One slot of a card's chrome: which [`ChromeElement`] renders and
/// at which [`ChromePlacement`].
///
/// `CardConfig::chrome` holds the ordered slot list; `None` derives it
/// from the legacy boolean flags (see `CardConfig::chrome`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ChromeSlot {
    /// The chrome element occupying this slot.
    pub element: ChromeElement,
    /// Where the element is mounted.
    pub placement: ChromePlacement,
}

impl ChromeSlot {
    /// The element mounted inline.
    pub const fn inline(element: ChromeElement) -> Self {
        Self {
            element,
            placement: ChromePlacement::Inline,
        }
    }

    /// The element mounted inline at the header's leading (`start`) end.
    pub const fn inline_start(element: ChromeElement) -> Self {
        Self {
            element,
            placement: ChromePlacement::InlineStart,
        }
    }

    /// The element mounted inline at the header's trailing (`end`) end.
    pub const fn inline_end(element: ChromeElement) -> Self {
        Self {
            element,
            placement: ChromePlacement::InlineEnd,
        }
    }

    /// The element mounted as a satellite.
    pub const fn satellite(element: ChromeElement) -> Self {
        Self {
            element,
            placement: ChromePlacement::Satellite,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors() {
        assert_eq!(ChromeSlot::inline(ChromeElement::Close).placement, ChromePlacement::Inline);
        assert_eq!(ChromeSlot::inline_start(ChromeElement::Close).placement, ChromePlacement::InlineStart);
        assert_eq!(ChromeSlot::inline_end(ChromeElement::Close).placement, ChromePlacement::InlineEnd);
        assert_eq!(ChromeSlot::satellite(ChromeElement::Close).placement, ChromePlacement::Satellite);
    }
}

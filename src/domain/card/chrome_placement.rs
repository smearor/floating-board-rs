/// Where a [`crate::domain::ChromeElement`] is mounted on a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum ChromePlacement {
    /// Inside the card chrome - header strip or content overlay,
    /// whichever the element's mount semantics dictate.
    #[default]
    Inline,
    /// As a satellite widget floating next to the card. Only elements
    /// with [`crate::domain::ChromeElement::supports_satellite`] may use
    /// this placement.
    Satellite,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_inline() {
        assert_eq!(ChromePlacement::default(), ChromePlacement::Inline);
    }
}

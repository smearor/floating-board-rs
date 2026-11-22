/// Where a [`crate::domain::ChromeElement`] is mounted on a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum ChromePlacement {
    /// Inside the card chrome at the renderer's default position -
    /// header strip or content overlay, whichever the element's mount
    /// semantics dictate. Action elements land on the trailing end.
    #[default]
    Inline,
    /// Inside the header strip at the leading (`start`) end - the left
    /// end in left-to-right direction, mirrored for RTL. Slotted order
    /// is kept: the first `InlineStart` slot sits outermost.
    InlineStart,
    /// Inside the header strip at the trailing (`end`) end - the right
    /// end in left-to-right direction. Slotted order is kept: the last
    /// `InlineEnd` slot sits outermost.
    InlineEnd,
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

    #[cfg(feature = "serde")]
    #[test]
    fn inline_ends_serialize_kebab_case() {
        let start = serde_json::to_string(&ChromePlacement::InlineStart).unwrap_or_default();
        let end = serde_json::to_string(&ChromePlacement::InlineEnd).unwrap_or_default();
        assert_eq!(start, r#""inline-start""#);
        assert_eq!(end, r#""inline-end""#);
    }
}

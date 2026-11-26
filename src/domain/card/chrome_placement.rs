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
    /// In the center region of a circle card's top arc band - the
    /// explicit counterpart to `Inline` once arc placements are used
    /// (see `arc_span` on `CardConfig`). Ignored on `Rectangle`.
    ArcTop,
    /// At the leading end of a circle card's top arc band - the card's
    /// left rim. `InlineStart` maps to the same zone. Ignored on
    /// `Rectangle`.
    ArcTopStart,
    /// At the trailing end of a circle card's top arc band - the
    /// card's right rim. `InlineEnd` maps to the same zone. Ignored
    /// on `Rectangle`.
    ArcTopEnd,
    /// In the center region of a circle card's bottom arc band - a
    /// second band mirrored at the disc's bottom. Ignored on
    /// `Rectangle`.
    ArcBottom,
    /// At the leading end of a circle card's bottom arc band - the
    /// band is mirrored, so the leading end sits on the screen's right
    /// side: a user facing the card's bottom reads start-to-end
    /// left-to-right. Ignored on `Rectangle`.
    ArcBottomStart,
    /// At the trailing end of a circle card's bottom arc band - the
    /// mirrored counterpart of `ArcTopEnd`. Ignored on `Rectangle`.
    ArcBottomEnd,
}

impl ChromePlacement {
    /// Whether the placement mounts on a circle card's top arc band
    /// (`Inline*` and `ArcTop*` placements).
    pub fn is_top_arc(&self) -> bool {
        matches!(self, Self::Inline | Self::InlineStart | Self::InlineEnd | Self::ArcTop | Self::ArcTopStart | Self::ArcTopEnd)
    }

    /// Whether the placement mounts on a circle card's bottom arc
    /// band (`ArcBottom*` placements).
    pub fn is_bottom_arc(&self) -> bool {
        matches!(self, Self::ArcBottom | Self::ArcBottomStart | Self::ArcBottomEnd)
    }
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

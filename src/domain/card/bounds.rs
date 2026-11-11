/// How strictly a card's footprint is kept inside the board bounds.
///
/// Configured per card via `CardConfig::bounds_mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum BoundsMode {
    /// Only `CardConfig::min_visible` pixels must stay inside the board -
    /// cards may partially leave the visible area.
    #[default]
    Loose,
    /// The full footprint must stay inside the board bounds at all times;
    /// drag, pinch and rotation clamp the card back immediately.
    Contained,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_mode_default_is_loose() {
        assert_eq!(BoundsMode::default(), BoundsMode::Loose);
    }
}

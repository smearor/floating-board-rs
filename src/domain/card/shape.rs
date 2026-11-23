/// The geometric outline a card renders and hit-tests with.
///
/// The widget hierarchy always stays rectangular - the shape decides how
/// the content is allocated inside the bounding box, how the card's
/// background is drawn and which points are pickable. On non-rectangular
/// shapes there is no header strip and no corner grips; chrome elements
/// mount as satellites instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum CardShape {
    /// The classic rectangular card: header strip plus rectangular content.
    #[default]
    Rectangle,
    /// A circular card: the bounding box is the circle's square, the
    /// content is allocated inside the circle (see [`CircleContent`]) and
    /// the ring between content and edge is the card's own rounded
    /// background. Taps in the bounding-box corners pass through to
    /// whatever is underneath.
    ///
    /// [`CircleContent`]: crate::CircleContent
    Circle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shape_is_rectangle() {
        assert_eq!(CardShape::default(), CardShape::Rectangle);
    }
}

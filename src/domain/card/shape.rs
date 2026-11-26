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
    /// A dome: the upper half of a circle - an arc on top, a flat
    /// diameter edge at the bottom. The bounding box is 2r wide and r
    /// tall; the content is allocated inside the dome (see
    /// [`CircleContent`], which also drives the dome's content
    /// region). Taps outside the dome pass through to whatever is
    /// underneath.
    ///
    /// Arc chrome mounts only on the top band (`Inline*`/`ArcTop*`
    /// placements); `Bar*` placements mount a straight bar along
    /// the flat bottom edge instead of an arc.
    ///
    /// [`CircleContent`]: crate::CircleContent
    Semicircle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shape_is_rectangle() {
        assert_eq!(CardShape::default(), CardShape::Rectangle);
    }
}

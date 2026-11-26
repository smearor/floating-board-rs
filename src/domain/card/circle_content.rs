use std::f64::consts::SQRT_2;

/// √5 - the semicircle's inscribed-square factor (`s = 2r/√5` for a
/// square bottom-aligned on the dome's flat edge).
const SQRT_5: f64 = 2.23606797749979;

/// How a [`CardShape::Circle`]/[`CardShape::Semicircle`] card
/// allocates its content inside the shape's bounding box.
///
/// [`CardShape::Circle`]: crate::CardShape
/// [`CardShape::Semicircle`]: crate::CardShape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum CircleContent {
    /// The content is allocated the square inscribed in the inner
    /// circle: `side = (d - 2·ring) / √2`. By construction the content
    /// can never paint or catch picks outside the circle - the card
    /// keeps the full ring as its own surface.
    #[default]
    Inscribed,
    /// The content is allocated the full inner square (`side = d -
    /// 2·ring`) and is expected to draw its own circular shape with
    /// transparent crescents (a clock face, a pie menu, a chart).
    /// Crescent picks inside the circle hit the content itself.
    Fill,
    /// Like [`CircleContent::Fill`], plus a circular clip on the
    /// content: rendering and picking stop at the inner circle, so
    /// crescent picks fall back onto the card - the ring stays a drag
    /// affordance even over a content that fills its square.
    Clipped,
}

impl CircleContent {
    /// The card diameter for `content_natural` - the larger of the
    /// content's natural width and height - given a `ring` band width
    /// (px) around the inner circle.
    ///
    /// For `Inscribed` the inner circle circumscribes the content
    /// square (`d_inner = side · √2`); for `Fill`/`Clipped` the inner
    /// circle is inscribed in it (`d_inner = side`).
    pub fn diameter_for(&self, content_natural: f64, ring: f64) -> f64 {
        let inner_diameter = match self {
            Self::Inscribed => content_natural * SQRT_2,
            Self::Fill | Self::Clipped => content_natural,
        };
        inner_diameter + 2.0 * ring
    }

    /// The content square's side length for a card of `diameter` with a
    /// ring band of `ring` (px). Inverse of [`Self::diameter_for`]:
    /// `content_square_for(diameter_for(nat)) == nat`.
    pub fn content_square_for(&self, diameter: f64, ring: f64) -> f64 {
        let inner = (diameter - 2.0 * ring).max(0.0);
        match self {
            Self::Inscribed => inner / SQRT_2,
            Self::Fill | Self::Clipped => inner,
        }
    }

    /// The dome's radius for a `Semicircle` card, given the content's
    /// natural `(width, height)` and a `ring` band width (px) inside
    /// the dome's edge.
    ///
    /// For `Inscribed` the inner dome circumscribes a square that is
    /// bottom-aligned on the flat edge (`r_inner = side·√5/2`); for
    /// `Fill`/`Clipped` the inner dome bounds a 2:1 content rect
    /// (`r_inner = max(width/2, height)`).
    pub fn dome_radius_for(&self, natural_width: f64, natural_height: f64, ring: f64) -> f64 {
        let inner = match self {
            Self::Inscribed => natural_width.max(natural_height) * SQRT_5 / 2.0,
            Self::Fill | Self::Clipped => (natural_width / 2.0).max(natural_height),
        };
        inner + ring
    }

    /// The content extent `(width, height)` inside a `Semicircle` card
    /// of `radius` with a ring band of `ring` (px). Inverse of
    /// [`Self::dome_radius_for`]. The content rect is bottom-aligned
    /// on the dome's flat edge: `Inscribed` gives the inscribed square
    /// (`side = 2·r_inner/√5`), `Fill`/`Clipped` the inner dome's
    /// bounding box (`2·r_inner × r_inner`).
    pub fn dome_content_for(&self, radius: f64, ring: f64) -> (f64, f64) {
        let inner = (radius - ring).max(0.0);
        match self {
            Self::Inscribed => {
                let side = 2.0 * inner / SQRT_5;
                (side, side)
            }
            Self::Fill | Self::Clipped => (2.0 * inner, inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_content_is_inscribed() {
        assert_eq!(CircleContent::default(), CircleContent::Inscribed);
    }

    #[test]
    fn inscribed_diameter_circumscribes_the_content_square() {
        // A 100px content square needs an inner circle of 100·√2 plus
        // a 4px ring on both sides.
        let diameter = CircleContent::Inscribed.diameter_for(100.0, 4.0);
        assert!((diameter - (100.0 * SQRT_2 + 8.0)).abs() < 1e-9);
    }

    #[test]
    fn fill_diameter_is_content_plus_ring() {
        let diameter = CircleContent::Fill.diameter_for(100.0, 4.0);
        assert!((diameter - 108.0).abs() < 1e-9);
        assert!((CircleContent::Clipped.diameter_for(100.0, 4.0) - 108.0).abs() < 1e-9);
    }

    #[test]
    fn diameter_and_square_are_inverses() {
        for content in [CircleContent::Inscribed, CircleContent::Fill, CircleContent::Clipped] {
            let diameter = content.diameter_for(120.0, 6.0);
            assert!((content.content_square_for(diameter, 6.0) - 120.0).abs() < 1e-9);
        }
    }

    #[test]
    fn degenerate_diameter_clamps_to_zero() {
        assert_eq!(CircleContent::Fill.content_square_for(4.0, 6.0), 0.0);
    }
}

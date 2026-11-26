use std::f64::consts::SQRT_2;

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
    /// natural `(width, height)`, a `ring` band width (px) inside the
    /// dome's edge and a `bottom_inset` (px) that lifts the content
    /// off the flat edge (the straight bar's band width, `0.0`
    /// without one).
    ///
    /// For `Inscribed` the inner dome circumscribes a square that is
    /// bottom-aligned `bottom_inset` above the flat edge: the binding
    /// corners are the square's top two, so
    /// `r_inner² = (side/2)² + (side + bottom_inset)²`. For
    /// `Fill`/`Clipped` the inner dome bounds a `2·r_inner ×
    /// (r_inner − bottom_inset)` content rect, so
    /// `r_inner = max(width/2, height + bottom_inset)`.
    pub fn dome_radius_for(&self, natural_width: f64, natural_height: f64, ring: f64, bottom_inset: f64) -> f64 {
        let inner = match self {
            Self::Inscribed => {
                let side = natural_width.max(natural_height);
                ((side / 2.0).powi(2) + (side + bottom_inset).powi(2)).sqrt()
            }
            Self::Fill | Self::Clipped => (natural_width / 2.0).max(natural_height + bottom_inset),
        };
        inner + ring
    }

    /// The content extent `(width, height)` inside a `Semicircle` card
    /// of `radius` with a ring band of `ring` (px) and a bottom inset
    /// of `bottom_inset` (px). Inverse of [`Self::dome_radius_for`].
    /// The content rect is bottom-aligned `bottom_inset` above the
    /// dome's flat edge: `Inscribed` solves the circumscribed-square
    /// quadratic for its side, `Fill`/`Clipped` gives the inner
    /// dome's bounding box (`2·r_inner × (r_inner − bottom_inset)`).
    pub fn dome_content_for(&self, radius: f64, ring: f64, bottom_inset: f64) -> (f64, f64) {
        let inner = (radius - ring).max(0.0);
        match self {
            Self::Inscribed => {
                // 1.25·side² + 2·bottom_inset·side + bottom_inset² = r_inner²
                let side = ((5.0 * inner.powi(2) - bottom_inset.powi(2)).max(0.0).sqrt() - 2.0 * bottom_inset) / 2.5;
                (side.max(0.0), side.max(0.0))
            }
            Self::Fill | Self::Clipped => (2.0 * inner, (inner - bottom_inset).max(0.0)),
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

    #[test]
    fn dome_radius_and_content_are_inverses() {
        // With no chrome the bottom inset vanishes and both helpers
        // reduce to the flat-edge formulas.
        for content in [CircleContent::Inscribed, CircleContent::Fill, CircleContent::Clipped] {
            for bottom_inset in [0.0, 20.0, 40.0] {
                let radius = content.dome_radius_for(150.0, 120.0, 8.0, bottom_inset);
                let (width, height) = content.dome_content_for(radius, 8.0, bottom_inset);
                assert!(width >= 150.0 - 1e-9, "{content:?} width {width} < 150 (inset {bottom_inset})");
                assert!(height >= 120.0 - 1e-9, "{content:?} height {height} < 120 (inset {bottom_inset})");
            }
        }
    }

    #[test]
    fn dome_bottom_inset_grows_the_radius() {
        // A bar under the content lifts it: the dome must grow so the
        // same square still fits above the bar.
        let flat = CircleContent::Inscribed.dome_radius_for(100.0, 100.0, 8.0, 0.0);
        let lifted = CircleContent::Inscribed.dome_radius_for(100.0, 100.0, 8.0, 36.0);
        assert!(lifted > flat);
        let (side, _) = CircleContent::Inscribed.dome_content_for(lifted, 8.0, 36.0);
        assert!((side - 100.0).abs() < 1e-9);
    }
}

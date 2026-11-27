use std::f64::consts::SQRT_2;

/// How a shaped card allocates its content inside the shape's
/// bounding box, in ascending extent: `Inscribed` < `Fill` <
/// `Clipped` < `Cover`.
///
/// Picking: GTK descends into children unconditionally and offers no
/// non-rectangular pick masks, so on non-rectangular shapes the
/// content subtree of `Fill`/`Clipped`/`Cover` is made non-targetable:
/// content in those modes is display-only and clicks in the cut
/// corners pass through to cards underneath. `Inscribed` and
/// `Rectangle` content stays interactive because its allocation never
/// covers the cut corners.
///
/// [`CardShape::Circle`]: crate::CardShape
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum CardContent {
    /// The content is allocated the square inscribed in the inner
    /// shape (`side = (d - 2·ring) / √2` on a `Circle`; the
    /// analogous inscribed square on polygons). By construction the
    /// content can never paint or catch picks outside the shape -
    /// the card keeps the full ring as its own surface.
    #[default]
    Inscribed,
    /// The content is allocated the full inner region (`side = d -
    /// 2·ring` on a `Circle`; the rim-shrunk silhouette's bounding
    /// box on polygons) and is expected to draw its own shape with
    /// transparent corners (a clock face, a pie menu, a chart).
    Fill,
    /// Like [`CardContent::Fill`], plus a silhouette clip on the
    /// content: rendering stops at the inner shape, so the cut
    /// regions show the card - the ring stays a visible drag
    /// affordance even over a content that fills its square.
    Clipped,
    /// The content is allocated the whole bounding box - the card's
    /// silhouette is the only bound, no ring stays card surface.
    /// The card's own silhouette clip is the bound on every shape,
    /// so a content that paints its square still shows the
    /// silhouette. Ignored for `Rectangle` (its bounding box is the
    /// silhouette).
    Cover,
}

impl CardContent {
    /// The card diameter for `content_natural` - the larger of the
    /// content's natural width and height - given a `ring` band width
    /// (px) around the inner circle.
    ///
    /// For `Inscribed` the inner circle circumscribes the content
    /// square (`d_inner = side · √2`); for `Fill`/`Clipped` the inner
    /// circle is inscribed in it (`d_inner = side`).
    pub fn diameter_for(&self, content_natural: f64, ring: f64) -> f64 {
        match self {
            Self::Inscribed => content_natural * SQRT_2 + 2.0 * ring,
            Self::Fill | Self::Clipped => content_natural + 2.0 * ring,
            // `Cover` fills the bounding box - no ring applies.
            Self::Cover => content_natural,
        }
    }

    /// The content square's side length for a card of `diameter` with a
    /// ring band of `ring` (px). Inverse of [`Self::diameter_for`]:
    /// `content_square_for(diameter_for(nat)) == nat`.
    pub fn content_square_for(&self, diameter: f64, ring: f64) -> f64 {
        let inner = (diameter - 2.0 * ring).max(0.0);
        match self {
            Self::Inscribed => inner / SQRT_2,
            Self::Fill | Self::Clipped => inner,
            Self::Cover => diameter,
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
    /// `r_inner = max(width/2, height + bottom_inset)`. For `Cover`
    /// the dome's bounding box `2r × r` holds the content, so
    /// `r = max(width/2, height)` and no ring applies.
    pub fn dome_radius_for(&self, natural_width: f64, natural_height: f64, ring: f64, bottom_inset: f64) -> f64 {
        match self {
            Self::Inscribed => {
                let side = natural_width.max(natural_height);
                ((side / 2.0).powi(2) + (side + bottom_inset).powi(2)).sqrt() + ring
            }
            Self::Fill | Self::Clipped => (natural_width / 2.0).max(natural_height + bottom_inset) + ring,
            Self::Cover => (natural_width / 2.0).max(natural_height),
        }
    }

    /// The content extent `(width, height)` inside a `Semicircle` card
    /// of `radius` with a ring band of `ring` (px) and a bottom inset
    /// of `bottom_inset` (px). Inverse of [`Self::dome_radius_for`].
    /// The content rect is bottom-aligned `bottom_inset` above the
    /// dome's flat edge: `Inscribed` solves the circumscribed-square
    /// quadratic for its side, `Fill`/`Clipped` gives the inner
    /// dome's bounding box (`2·r_inner × (r_inner − bottom_inset)`),
    /// `Cover` the dome's bounding box (`2r × r`).
    pub fn dome_content_for(&self, radius: f64, ring: f64, bottom_inset: f64) -> (f64, f64) {
        let inner = (radius - ring).max(0.0);
        match self {
            Self::Inscribed => {
                // 1.25·side² + 2·bottom_inset·side + bottom_inset² = r_inner²
                let side = ((5.0 * inner.powi(2) - bottom_inset.powi(2)).max(0.0).sqrt() - 2.0 * bottom_inset) / 2.5;
                (side.max(0.0), side.max(0.0))
            }
            Self::Fill | Self::Clipped => (2.0 * inner, (inner - bottom_inset).max(0.0)),
            Self::Cover => (2.0 * radius, radius),
        }
    }
}

/// Compat alias for the pre-rename name: `CircleContent` is
/// `CardContent` - the fill modes apply to every shaped card.
pub type CircleContent = CardContent;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_content_is_inscribed() {
        assert_eq!(CardContent::default(), CardContent::Inscribed);
    }

    #[test]
    fn inscribed_diameter_circumscribes_the_content_square() {
        // A 100px content square needs an inner circle of 100·√2 plus
        // a 4px ring on both sides.
        let diameter = CardContent::Inscribed.diameter_for(100.0, 4.0);
        assert!((diameter - (100.0 * SQRT_2 + 8.0)).abs() < 1e-9);
    }

    #[test]
    fn fill_diameter_is_content_plus_ring() {
        let diameter = CardContent::Fill.diameter_for(100.0, 4.0);
        assert!((diameter - 108.0).abs() < 1e-9);
        assert!((CardContent::Clipped.diameter_for(100.0, 4.0) - 108.0).abs() < 1e-9);
    }

    #[test]
    fn diameter_and_square_are_inverses() {
        for content in [CardContent::Inscribed, CardContent::Fill, CardContent::Clipped, CardContent::Cover] {
            let diameter = content.diameter_for(120.0, 6.0);
            assert!((content.content_square_for(diameter, 6.0) - 120.0).abs() < 1e-9);
        }
    }

    #[test]
    fn degenerate_diameter_clamps_to_zero() {
        assert_eq!(CardContent::Fill.content_square_for(4.0, 6.0), 0.0);
    }

    #[test]
    fn dome_radius_and_content_are_inverses() {
        // With no chrome the bottom inset vanishes and both helpers
        // reduce to the flat-edge formulas.
        for content in [CardContent::Inscribed, CardContent::Fill, CardContent::Clipped, CardContent::Cover] {
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
        let flat = CardContent::Inscribed.dome_radius_for(100.0, 100.0, 8.0, 0.0);
        let lifted = CardContent::Inscribed.dome_radius_for(100.0, 100.0, 8.0, 36.0);
        assert!(lifted > flat);
        let (side, _) = CardContent::Inscribed.dome_content_for(lifted, 8.0, 36.0);
        assert!((side - 100.0).abs() < 1e-9);
    }
}

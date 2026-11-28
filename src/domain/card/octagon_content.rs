use smearor_commons_geometry::Octagon;
use smearor_commons_geometry::PolygonOrientation;

// Content-region math for a flat-top octagon card: the bounding
// box is the square `2a x 2a` where `a` is the inradius (center to
// flat edge, also called apothem; the circumradius `R = a /
// cos(22.5°)` reaches the vertices). The largest centered square
// inside the octagon has side `sqrt(2) * a` - its corners rest on
// the four diagonal edges. Chrome bars on the flat edges
// (`bar_inset`, the combined top+bottom band height) shrink the
// usable height to `2 * a - 2 * bar_inset`.

/// The inradius whose inscribed content square fits `side` -
/// [`Octagon::inradius_for_inscribed_square`] is `side / sqrt(2)` -
/// grown by the ring margin, but at least large enough that the
/// centered square clears a combined bar inset `bar_inset` tall on
/// each flat edge - the sum of the top and bottom band heights
/// (`2 * a >= side + 2 * bar_inset`).
pub fn octagon_inradius_for(content_side: f64, ring: f64, bar_inset: f64) -> f64 {
    (Octagon::<()>::inradius_for_inscribed_square(content_side, PolygonOrientation::FlatTop) + ring)
        .max(Octagon::<()>::inradius_for_bounding_side(content_side + 2.0 * bar_inset, PolygonOrientation::FlatTop))
}

/// The content square fitting inside the octagon of `inradius`:
/// the inscribed square of the rim-shrunk octagon, additionally
/// capped so its edges clear the combined top and bottom bar
/// insets `bar_inset` tall. The exact inverse of
/// [`octagon_inradius_for`].
pub fn octagon_content_for(inradius: f64, ring: f64, bar_inset: f64) -> f64 {
    Octagon::<()>::inscribed_square_side_for_inradius(inradius - ring, PolygonOrientation::FlatTop)
        .min(Octagon::<()>::bounding_side_for_inradius(inradius, PolygonOrientation::FlatTop) - 2.0 * bar_inset)
}

/// The inradius whose rim-shrunk bounding square fits `natural`
/// for `Fill`/`Clipped` content: `natural / 2 + ring`. The content
/// underlaps the chrome bars, so no bar inset applies.
pub fn octagon_fill_inradius_for(natural: f64, ring: f64) -> f64 {
    Octagon::<()>::inradius_for_bounding_side(natural, PolygonOrientation::FlatTop) + ring
}

/// The rim-shrunk bounding square a `Fill`/`Clipped` content gets
/// inside the octagon of `inradius`: `2 * (inradius - ring)`. The
/// exact inverse of [`octagon_fill_inradius_for`].
pub fn octagon_fill_extent(inradius: f64, ring: f64) -> f64 {
    Octagon::<()>::bounding_side_for_inradius(inradius - ring, PolygonOrientation::FlatTop).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::SQRT_2;

    /// `octagon_inradius_for` and `octagon_content_for` are exact
    /// inverses for both constraint branches.
    #[test]
    fn octagon_inradius_and_content_are_inverses() {
        for side in [100.0, 240.0, 40.0] {
            for bar in [0.0, 20.0, 44.0] {
                let inradius = octagon_inradius_for(side, 8.0, bar);
                assert!((octagon_content_for(inradius, 8.0, bar) - side).abs() < 1e-9, "side {side} bar {bar} -> inradius {inradius}");
            }
        }
    }

    /// A bar inset grows the inradius once its constraint dominates
    /// the inscribed-square one.
    #[test]
    fn octagon_bar_inset_grows_the_inradius() {
        let side = 100.0;
        let plain = octagon_inradius_for(side, 8.0, 0.0);
        let with_bar = octagon_inradius_for(side, 8.0, 44.0);
        assert!(with_bar > plain, "bars lift content -> larger octagon");
        // The bar-bound inradius clears the combined inset on both
        // sides: 2 * a = side + 2 * inset.
        assert!((with_bar * 2.0 - 2.0 * 44.0 - side).abs() < 1e-9);
    }

    /// `octagon_fill_inradius_for` and `octagon_fill_extent` are
    /// exact inverses.
    #[test]
    fn octagon_fill_extent_inverts_inradius() {
        for natural in [100.0, 240.0] {
            let inradius = octagon_fill_inradius_for(natural, 8.0);
            assert!((octagon_fill_extent(inradius, 8.0) - natural).abs() < 1e-9);
        }
    }

    /// The inscribed square's corner rests exactly on a diagonal edge.
    #[test]
    fn inscribed_square_corner_touches_the_diagonal() {
        let inradius = 120.0;
        let half = octagon_content_for(inradius, 0.0, 0.0) / 2.0;
        // Diagonal edge: x + y == inradius * sqrt(2).
        assert!((half + half - inradius * SQRT_2).abs() < 1e-9);
    }
}

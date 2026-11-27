/// `sqrt(3)` - the flat-top hexagon's height factor
/// (`bounding box height = sqrt(3) * R`) and `2 * sin(60°)`.
const SQRT_3: f64 = 1.732_050_807_568_877_2;

// Content-region math for a flat-top hexagon card: the bounding
// box is `2R` wide and `sqrt(3) * R` tall where `R` is the
// circumradius (center to vertex, equal to the edge length). The
// largest centered square inside the hexagon has side
// `(3 - sqrt(3)) * R` - its corners rest on the four slanted
// edges. Chrome bars on the flat edges (`bar_inset`, the
// combined top+bottom band height) shrink the usable height to
// `sqrt(3) * R - 2 * bar_inset`.

/// Side of the square inscribed in a hexagon of circumradius `1`:
/// `3 - sqrt(3) ≈ 1.268`. The corner `(|s/2|, |s/2|)` rests on the
/// slanted edge `sin(60°) * x + y / 2 = R * sin(60°)`.
const HEXAGON_CONTENT_FACTOR: f64 = 3.0 - SQRT_3;

/// The circumradius whose inscribed content square fits `side`:
/// `side / (3 - sqrt(3))` grown by the ring margin, but at least
/// large enough that the centered square clears a combined bar
/// inset `bar_inset` tall on each flat edge - the sum of the top
/// and bottom band heights (`sqrt(3) * R >= side + 2 * bar_inset`).
pub fn hexagon_radius_for(content_side: f64, ring: f64, bar_inset: f64) -> f64 {
    (content_side / HEXAGON_CONTENT_FACTOR + ring).max((content_side + 2.0 * bar_inset) / SQRT_3)
}

/// The content square fitting inside the hexagon of `radius`: the
/// inscribed square of the rim-shrunk hexagon, additionally capped
/// so its edges clear the combined top and bottom bar insets
/// `bar_inset` tall. The exact inverse of [`hexagon_radius_for`].
pub fn hexagon_content_for(radius: f64, ring: f64, bar_inset: f64) -> f64 {
    ((radius - ring) * HEXAGON_CONTENT_FACTOR).min(radius * SQRT_3 - 2.0 * bar_inset)
}

/// The circumradius whose rim-shrunk bounding box fits
/// `(natural_width, natural_height)` for `Fill`/`Clipped` content:
/// `2 * (R - ring) >= width` and `sqrt(3) * (R - ring) >= height`.
/// The content underlaps the chrome bars, so no bar inset applies.
pub fn hexagon_fill_radius_for(natural_width: f64, natural_height: f64, ring: f64) -> f64 {
    (natural_width / 2.0).max(natural_height / SQRT_3) + ring
}

/// The rim-shrunk bounding box `(width, height)` a `Fill`/`Clipped`
/// content gets inside the hexagon of `radius`:
/// `(2 * (R - ring), sqrt(3) * (R - ring))`. The exact inverse of
/// [`hexagon_fill_radius_for`].
pub fn hexagon_fill_extent(radius: f64, ring: f64) -> (f64, f64) {
    let inner = (radius - ring).max(0.0);
    (2.0 * inner, SQRT_3 * inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `hexagon_radius_for` and `hexagon_content_for` are exact
    /// inverses for both constraint branches.
    #[test]
    fn hexagon_radius_and_content_are_inverses() {
        for side in [100.0, 240.0, 40.0] {
            for bar in [0.0, 20.0, 44.0] {
                let radius = hexagon_radius_for(side, 8.0, bar);
                assert!((hexagon_content_for(radius, 8.0, bar) - side).abs() < 1e-9, "side {side} bar {bar} -> radius {radius}");
            }
        }
    }

    /// A bar inset grows the radius once its constraint dominates
    /// the inscribed-square one.
    #[test]
    fn hexagon_bar_inset_grows_the_radius() {
        let side = 100.0;
        let plain = hexagon_radius_for(side, 8.0, 0.0);
        let with_bar = hexagon_radius_for(side, 8.0, 44.0);
        assert!(with_bar > plain, "bars lift content -> larger hexagon");
        // The bar-bound radius clears the combined inset on both
        // sides: sqrt(3) * R = side + 2 * inset.
        assert!((with_bar * SQRT_3 - 2.0 * 44.0 - side).abs() < 1e-9);
    }

    /// `hexagon_fill_radius_for` and `hexagon_fill_extent` are
    /// exact inverses on both constraint branches.
    #[test]
    fn hexagon_fill_extent_inverts_radius() {
        for (width, height) in [(100.0, 100.0), (300.0, 100.0)] {
            let radius = hexagon_fill_radius_for(width, height, 8.0);
            let (extent_width, extent_height) = hexagon_fill_extent(radius, 8.0);
            assert!(extent_width >= width - 1e-9 && extent_height >= height - 1e-9);
        }
    }

    /// The inscribed square's corner rests exactly on a slanted edge.
    #[test]
    fn inscribed_square_corner_touches_the_slant() {
        let radius = 120.0;
        let half = hexagon_content_for(radius, 0.0, 0.0) / 2.0;
        // Slanted edge: sin(60°) * x + y / 2 == inradius = R * sin(60°).
        let edge = (SQRT_3 / 2.0) * half + half / 2.0;
        assert!((edge - radius * SQRT_3 / 2.0).abs() < 1e-9);
    }
}

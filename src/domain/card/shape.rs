use smearor_commons_geometry::Distance;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Vec2;

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

impl CardShape {
    /// How far the shape's outline extends along `direction`, measured
    /// from the bounding box's center: the disk radius for a `Circle`
    /// card, the axis-aligned box support `(|cos| * w + |sin| * h) / 2`
    /// for a `Rectangle` card (exact for the bbox - the visible
    /// corners of a rotated card slightly overshoot, which is fine
    /// for an emerge seed). A `Semicircle` card orbits like its full
    /// circle - the bbox's inscribed radius, a uniform gap all
    /// around.
    pub fn contour_support(&self, direction: RotationRad, footprint: Vec2<Pixels>) -> Distance<Pixels> {
        match self {
            Self::Circle | Self::Semicircle => Distance::new(footprint.x / 2.0),
            Self::Rectangle => {
                let (sin, cos) = direction.sin_cos();
                Distance::new((cos.abs() * footprint.x + sin.abs() * footprint.y) / 2.0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_shape_is_rectangle() {
        assert_eq!(CardShape::default(), CardShape::Rectangle);
    }

    #[test]
    fn circle_support_is_the_disk_radius_everywhere() {
        let footprint = Vec2::<Pixels>::new(200.0, 200.0);
        for direction in [RotationRad::ZERO, RotationRad::QUARTER_TURN, RotationRad::new(1.1)] {
            assert_eq!(CardShape::Circle.contour_support(direction, footprint).as_f64(), 100.0);
        }
    }

    #[test]
    fn rectangle_support_follows_the_box_contour() {
        let footprint = Vec2::<Pixels>::new(200.0, 100.0);
        // Straight down: the support reaches the bottom edge (h/2).
        assert!((CardShape::Rectangle.contour_support(RotationRad::QUARTER_TURN, footprint).as_f64() - 50.0).abs() < 1e-9);
        // Diagonal (cos=0.8, sin=0.6): (0.8*200 + 0.6*100) / 2 = 110.
        let direction = RotationRad::new(0.6435);
        assert!((CardShape::Rectangle.contour_support(direction, footprint).as_f64() - 110.0).abs() < 1.0);
    }

    #[test]
    fn semicircle_support_matches_the_full_circle() {
        let footprint = Vec2::<Pixels>::new(200.0, 100.0);
        assert_eq!(CardShape::Semicircle.contour_support(RotationRad::QUARTER_TURN, footprint).as_f64(), 100.0);
    }
}

use smearor_commons_geometry::Distance;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;
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

    /// The radius of the centered circle the shape's outline fits
    /// into: the disk radius for `Circle`, the bounding box's
    /// circumscribed circle (`magnitude / 2`) for `Rectangle`, and
    /// the dome's virtual-circle radius for `Semicircle` (the flat
    /// edge's corners overshoot it by ~12% - the same uniform
    /// approximation [`Self::contour_support`] documents).
    pub fn bounding_radius(&self, footprint: Vec2<Pixels>) -> Distance<Pixels> {
        match self {
            Self::Circle | Self::Semicircle => Distance::new(footprint.x / 2.0),
            Self::Rectangle => Distance::new(footprint.magnitude_f64() / 2.0),
        }
    }

    /// The extent a folded card of this shape pins to, derived from
    /// the content's `natural` size: a `Circle` collapses onto its
    /// ring, so the diameter (the larger natural axis) stays pinned
    /// as a square; a `Semicircle` dome keeps its `2r × r`
    /// proportion; a `Rectangle` pins the width only. A `-1` height
    /// means the folded measure decides it.
    pub fn collapsed_extent(&self, natural: Size<i32>) -> Size<i32> {
        match self {
            Self::Circle => {
                let diameter = natural.x.max(natural.y);
                Size::new(diameter, diameter)
            }
            Self::Semicircle => {
                let diameter = natural.x.max(natural.y);
                Size::new(diameter, diameter / 2)
            }
            Self::Rectangle => Size::new(natural.x, -1),
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
    fn bounding_radius_circumscribes_the_shape() {
        // Circle/dome: the disk's own radius. Rectangle: the
        // circumscribed circle sqrt(200^2 + 100^2) / 2.
        let square = Vec2::<Pixels>::new(200.0, 200.0);
        let rect = Vec2::<Pixels>::new(200.0, 100.0);
        assert_eq!(CardShape::Circle.bounding_radius(square).as_f64(), 100.0);
        assert_eq!(CardShape::Semicircle.bounding_radius(rect).as_f64(), 100.0);
        assert!((CardShape::Rectangle.bounding_radius(rect).as_f64() - 111.803).abs() < 0.001);
    }

    #[test]
    fn semicircle_support_matches_the_full_circle() {
        let footprint = Vec2::<Pixels>::new(200.0, 100.0);
        assert_eq!(CardShape::Semicircle.contour_support(RotationRad::QUARTER_TURN, footprint).as_f64(), 100.0);
    }

    #[test]
    fn collapsed_extent_keeps_each_shape_silhouette() {
        let natural = Size::new(200, 120);
        // Circle: square pin on the larger axis; dome: 2r x r;
        // rectangle: width only (-1 = the folded measure decides).
        assert_eq!(CardShape::Circle.collapsed_extent(natural), Size::new(200, 200));
        assert_eq!(CardShape::Semicircle.collapsed_extent(natural), Size::new(200, 100));
        assert_eq!(CardShape::Rectangle.collapsed_extent(natural), Size::new(200, -1));
    }
}

use crate::domain::CardOutline;
use smearor_commons_geometry::Annulus;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::Distance;
use smearor_commons_geometry::EighthTurn;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Hexagon;
use smearor_commons_geometry::Octagon;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::PolygonOrientation;
use smearor_commons_geometry::QuarterTurn;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Rotation;
use smearor_commons_geometry::RotationDirection;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Semicircle;
use smearor_commons_geometry::SixthTurn;
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
    /// content is allocated inside the circle (see [`CardContent`]) and
    /// the ring between content and edge is the card's own rounded
    /// background. Taps in the bounding-box corners pass through to
    /// whatever is underneath.
    ///
    /// [`CardContent`]: crate::CircleContent
    Circle,
    /// A dome: the upper half of a circle - an arc on top, a flat
    /// diameter edge at the bottom. The bounding box is 2r wide and r
    /// tall; the content is allocated inside the dome (see
    /// [`CardContent`], which also drives the dome's content
    /// region). Taps outside the dome pass through to whatever is
    /// underneath.
    ///
    /// Arc chrome mounts only on the top band (`Inline*`/`ArcTop*`
    /// placements); `Bar*` placements mount a straight bar along
    /// the flat bottom edge instead of an arc.
    ///
    /// [`CardContent`]: crate::CircleContent
    Semicircle,
    /// A flat-top hexagon: horizontal top and bottom edges, slanted
    /// left/right edges meeting in vertices. The bounding box is
    /// `2R` wide and `sqrt(3) * R` tall (`R` = circumradius); the
    /// content lives in the inscribed square (see
    /// `hexagon_content_for`). Taps in the bounding-box corners pass
    /// through to whatever is underneath.
    ///
    /// There is no header strip and no corner grips; chrome elements
    /// mount as a straight bar along the flat bottom edge.
    Hexagon,
    /// A flat-top octagon: horizontal/vertical flat edges, diagonal
    /// edges cutting the four corners at 45 degrees. The bounding
    /// box is the square `2a x 2a` (`a` = inradius, center to flat
    /// edge); the content lives in the inscribed square (see
    /// `octagon_content_for`). Taps in the bounding-box corners pass
    /// through to whatever is underneath.
    ///
    /// There is no header strip and no corner grips; chrome elements
    /// mount as straight bars along the flat top/bottom edges.
    Octagon,
    /// A ring / annulus: the area between the outer rim and the
    /// see-through hole (`CardConfig::hole_ratio` of the outer
    /// radius). The bounding box is the outer circle's square; the
    /// hole shows whatever is underneath and taps inside it pass
    /// through - the card surface lives on the band only.
    ///
    /// There is no header strip and no corner grips; chrome elements
    /// mount along the ring's arc bands like on a `Circle`.
    Annulus,
}

/// `sqrt(3) / 2` - a flat-top hexagon's height is `sqrt(3) * R`
/// for circumradius `R`, so the folded pin height is
/// `diameter * sqrt(3) / 2`.
const SQRT_3_OVER_2: f64 = 0.866_025_403_784_438_6;

impl CardShape {
    /// How far the shape's outline extends along `direction`, measured
    /// from the bounding box's center: the disk radius for a `Circle`
    /// card, the axis-aligned box support `(|cos| * w + |sin| * h) / 2`
    /// for a `Rectangle` card (exact for the bbox - the visible
    /// corners of a rotated card slightly overshoot, which is fine
    /// for an emerge seed). A `Semicircle` card orbits like its full
    /// circle - the bbox's inscribed radius, a uniform gap all
    /// around. A `Hexagon` card reports the exact support of the
    /// flat-top hexagon (the largest vertex projection along the
    /// direction); an `Octagon` card reports its exact support
    /// likewise.
    pub fn contour_support(&self, direction: RotationRad, footprint: Vec2<Pixels>) -> Distance<Pixels> {
        match self {
            Self::Circle | Self::Semicircle | Self::Annulus => Distance::new(footprint.x / 2.0),
            Self::Rectangle => {
                let (sin, cos) = direction.sin_cos();
                Distance::new((cos.abs() * footprint.x + sin.abs() * footprint.y) / 2.0)
            }
            Self::Hexagon => Distance::new(Hexagon::flat_top(Vec2::<Pixels>::new(footprint.x / 2.0, footprint.y / 2.0), footprint.x / 2.0).support(direction)),
            Self::Octagon => Distance::new(
                Octagon::with_inradius(Vec2::<Pixels>::new(footprint.x / 2.0, footprint.y / 2.0), footprint.x / 2.0, PolygonOrientation::FlatTop)
                    .support(direction),
            ),
        }
    }

    /// The radius of the centered circle the shape's outline fits
    /// into: the disk radius for `Circle`, the bounding box's
    /// circumscribed circle (`magnitude / 2`) for `Rectangle`, and
    /// the dome's virtual-circle radius for `Semicircle` (the flat
    /// edge's corners overshoot it by ~12% - the same uniform
    /// approximation [`Self::contour_support`] documents). For a
    /// `Hexagon` card the circumradius is exact - every vertex sits
    /// on it. For an `Octagon` the vertices reach `a / cos(22.5°)`,
    /// slightly past the bounding square's half extent.
    pub fn bounding_radius(&self, footprint: Vec2<Pixels>) -> Distance<Pixels> {
        match self {
            Self::Circle | Self::Semicircle | Self::Hexagon | Self::Annulus => Distance::new(footprint.x / 2.0),
            Self::Octagon => Distance::new(
                Octagon::with_inradius(Vec2::<Pixels>::new(footprint.x / 2.0, footprint.y / 2.0), footprint.x / 2.0, PolygonOrientation::FlatTop).circumradius,
            ),
            Self::Rectangle => Distance::new(footprint.magnitude_f64() / 2.0),
        }
    }

    /// The rotation snap target one step past `current` in
    /// `direction`: a `Hexagon` steps in `SixthTurn`s - 60 degrees,
    /// its sixfold symmetry - an `Octagon` in `EighthTurn`s (45
    /// degrees), every other shape in `QuarterTurn`s. The typed
    /// `Rotation` variant carries the shape's symmetry.
    pub fn rotation_snap_target(&self, current: RotationRad, direction: RotationDirection) -> Rotation {
        match self {
            CardShape::Hexagon => SixthTurn::snap_target(current, direction).into(),
            CardShape::Octagon => EighthTurn::snap_target(current, direction).into(),
            _ => QuarterTurn::snap_target(current, direction).into(),
        }
    }

    /// The angular distance between the shape's rotation snap
    /// boundaries: 60 degrees for a `Hexagon`, 45 degrees for an
    /// `Octagon`, 90 degrees for every other shape - the
    /// drag-rotation counterpart of [`Self::rotation_snap_target`].
    pub fn rotation_step(&self) -> Degrees {
        match self {
            CardShape::Hexagon => Degrees::SIXTH_TURN,
            CardShape::Octagon => Degrees::EIGHTH_TURN,
            _ => Degrees::QUARTER_TURN,
        }
    }

    /// The shape's geometry inscribed in `bounds`: the disc for
    /// `Circle`, the dome (center at the bounds' bottom middle) for
    /// `Semicircle`, the flat-top polygons for `Hexagon`/`Octagon` and
    /// the ring with `hole` as its inner-radius share for `Annulus`.
    /// `Rectangle` has no inscribed outline - its contour is the
    /// bounding box itself - so callers fall back to rectangular
    /// defaults (`parent_contains` for hit-tests, no silhouette clip
    /// path for snapshots).
    pub fn outline(&self, bounds: &Rect<()>, hole: Fraction) -> Option<CardOutline> {
        let outline = match self {
            Self::Circle => CardOutline::Circle(Circle::inscribed_in(bounds)),
            Self::Semicircle => CardOutline::Semicircle(Semicircle::inscribed_in(bounds)),
            Self::Hexagon => CardOutline::Hexagon(Hexagon::inscribed_in(bounds, PolygonOrientation::FlatTop)),
            Self::Octagon => CardOutline::Octagon(Octagon::inscribed_in(bounds, PolygonOrientation::FlatTop)),
            Self::Annulus => CardOutline::Annulus(Annulus::inscribed_in(bounds, hole)),
            Self::Rectangle => return None,
        };
        Some(outline)
    }

    /// Whether `point` hits the shape's contour inside `bounds` -
    /// `None` for `Rectangle`, whose callers keep the rectangular
    /// fallback test.
    pub fn contains(&self, bounds: &Rect<()>, hole: Fraction, point: &Vec2<()>) -> Option<bool> {
        self.outline(bounds, hole).map(|outline| outline.contains(bounds, point))
    }

    /// The radius the arc chrome rides at: an `Annulus` ring floats
    /// its chrome on the band's middle (`(inner + outer) / 2`); every
    /// other shape rides the inscribed circle's rim minus half the
    /// `band` thickness.
    pub fn arc_chrome_radius(&self, bounds: &Rect<()>, hole: Fraction, band: f64) -> f64 {
        match self.outline(bounds, hole) {
            Some(CardOutline::Annulus(annulus)) => (annulus.inner + annulus.outer) / 2.0,
            _ => bounds.size.x.min(bounds.size.y) / 2.0 - band / 2.0,
        }
    }

    /// The extent a folded card of this shape pins to, derived from
    /// the content's `natural` size: a `Circle` collapses onto its
    /// ring, so the diameter (the larger natural axis) stays pinned
    /// as a square; a `Semicircle` dome keeps its `2r × r`
    /// proportion; a `Hexagon` keeps its `2R × sqrt(3)·R`
    /// proportion; a `Rectangle` pins the width only. A `-1` height
    /// means the folded measure decides it.
    pub fn collapsed_extent(&self, natural: Size<i32>) -> Size<i32> {
        match self {
            Self::Circle | Self::Annulus => {
                let diameter = natural.x.max(natural.y);
                Size::new(diameter, diameter)
            }
            Self::Semicircle => {
                let diameter = natural.x.max(natural.y);
                Size::new(diameter, diameter / 2)
            }
            Self::Hexagon => {
                let diameter = natural.x.max(natural.y);
                Size::new(diameter, (f64::from(diameter) * SQRT_3_OVER_2).ceil() as i32)
            }
            Self::Octagon => {
                let diameter = natural.x.max(natural.y);
                Size::new(diameter, diameter)
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
        // Hexagon: 2R x sqrt(3)R pin on the larger axis.
        assert_eq!(CardShape::Hexagon.collapsed_extent(natural), Size::new(200, 174));
    }

    #[test]
    fn hexagon_support_reaches_the_vertices_exactly() {
        let footprint = Vec2::<Pixels>::new(200.0, 174.0);
        // Along +x the support is the circumradius (a vertex sits
        // there); straight up it is the inradius R * sqrt(3)/2.
        assert!((CardShape::Hexagon.contour_support(RotationRad::ZERO, footprint).as_f64() - 100.0).abs() < 1e-9);
        assert!((CardShape::Hexagon.contour_support(RotationRad::QUARTER_TURN, footprint).as_f64() - 86.6).abs() < 0.01);
        assert_eq!(CardShape::Hexagon.bounding_radius(footprint).as_f64(), 100.0);
    }
}

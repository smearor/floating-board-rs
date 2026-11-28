use smearor_commons_geometry::Annulus;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Hexagon;
use smearor_commons_geometry::Octagon;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Semicircle;
use smearor_commons_geometry::Vec2;

/// The geometry a [`CardShape`] inscribes in its bounding box - the
/// commons-geometry value behind every silhouette, hit-test and
/// chrome-band computation. `Rectangle` has no inscribed outline (its
/// contour is the bounding box itself), so [`CardShape::outline`]
/// returns `None` for it.
///
/// [`CardShape`]: crate::domain::CardShape
/// [`CardShape::outline`]: crate::domain::CardShape::outline
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CardOutline {
    /// The disc inscribed in the bounding square.
    Circle(Circle<()>),
    /// The dome: the upper half of the virtual circle (center at the
    /// bounding box's bottom middle) - the flat edge cuts the circle's
    /// lower half, so probes below the allocation never re-enter.
    Semicircle(Semicircle<()>),
    /// The flat-top hexagon inscribed in the bounding box.
    Hexagon(Hexagon<()>),
    /// The flat-top octagon inscribed in the bounding square.
    Octagon(Octagon<()>),
    /// The ring: the inscribed outer circle minus its `hole` share.
    Annulus(Annulus<()>),
}

impl CardOutline {
    /// Whether `point` hits the outline's contour, guarded by
    /// `bounds`: the shape test is exact inside the bounding box; the
    /// bounds guard keeps the rotation wrapper's out-of-box probes
    /// from re-entering the shape (tautological for the inscribed
    /// shapes - they are subsets of their bounds by construction).
    pub fn contains(&self, bounds: &Rect<()>, point: &Vec2<()>) -> bool {
        bounds.contains(point)
            && match self {
                Self::Circle(circle) => circle.contains(point),
                Self::Semicircle(dome) => dome.contains(point),
                Self::Hexagon(hexagon) => hexagon.contains(point),
                Self::Octagon(octagon) => octagon.contains(point),
                Self::Annulus(annulus) => annulus.contains(point),
            }
    }
}

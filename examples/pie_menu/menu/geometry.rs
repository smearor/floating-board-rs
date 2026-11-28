//! The pie menu's ring geometry - shared by hit-testing and drawing.

use super::items::ITEMS;
use smearor_commons_geometry::Annulus;
use smearor_commons_geometry::Arc;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Vec2;
use std::f64::consts::FRAC_PI_2;
use std::f64::consts::TAU;

/// The hole's fraction of the outer radius - a wide band leaves room
/// for labels between hole and rim.
pub(crate) const HOLE_RATIO: Fraction = Fraction::new(0.4);
/// Angular gap between neighboring wedges.
pub(super) const WEDGE_GAP: RotationRad = RotationRad::new(0.035);

/// The menu's ring in face space - same geometry for hit-testing and
/// drawing.
pub(super) fn menu_annulus(size: Vec2<Pixels>) -> Annulus<Pixels> {
    Annulus::inscribed_in(&Rect::<Pixels>::from_size(size), HOLE_RATIO)
}

/// Wedge `index`'s angular interval. Items are laid out from the top
/// clockwise; `FillSector`'s cairo backend feeds the `Arc` bounds
/// straight into `Context::arc`, whose y-down space already measures
/// angles clockwise from +x - so the arc's `start` is simply the
/// wedge's screen angle offset by the top at -90deg.
pub(super) fn wedge_arc(index: usize) -> Arc {
    let step = TAU / ITEMS.len() as f64;
    Arc::new(RotationRad::new(index as f64 * step - FRAC_PI_2) + WEDGE_GAP * 0.5, RotationRad::new(step) - WEDGE_GAP)
}

/// Maps a face-space point to a wedge index: the point must lie on
/// the band (`r` between hole and outer) and inside a wedge, not in
/// the angular gap between two.
pub(crate) fn wedge_at(point: Vec2<Pixels>, size: Vec2<Pixels>) -> Option<usize> {
    let annulus = menu_annulus(size);
    if !annulus.contains(&point) {
        return None;
    }
    // `direction` reads the point's angle in the same convention the
    // wedges use - `Vec2::direction` on the y-down face space is the
    // cairo angle `Arc::contains` tests.
    let direction = (point - annulus.center).direction();
    (0..ITEMS.len()).find(|&index| wedge_arc(index).contains(direction))
}

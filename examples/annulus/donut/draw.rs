//! The `donut` face's drawing.

use super::color::DonutColor;
use super::color::donut_color;
use crate::HOLE_RATIO;
use gtk4::DrawingArea;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillRect;
use smearor_commons_drawing::FillSectors;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Annulus;
use smearor_commons_geometry::Arc;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;

/// `Fill` face: a six-slice donut painted to the square's bounds -
/// the corners of the square visibly paint over the hole, showing
/// that `Fill` bounds (not the silhouette) are the limit.
pub(crate) fn build_donut(size: Pixels) -> DrawingArea {
    drawing_target(Some(Size::new(f64::from(size) as i32, f64::from(size) as i32)), move |target| {
        let bounds = Rect::<Pixels>::from_size(target.size().to_f64().with_unit());
        let face = Circle::inscribed_in(&bounds);
        // Painted square corners: `Fill` lets them escape into the
        // hole and the cut regions.
        let _ = target.fill_rect(FillRect::new(bounds, donut_color(DonutColor::Backdrop), 0.55));
        let _ = target.fill_sectors(FillSectors::new(
            Annulus::new(face.center, face.radius * HOLE_RATIO.as_f64(), face.radius),
            Arc::new(RotationRad::ZERO, RotationRad::FULL_TURN),
            6,
            RotationRad::ZERO,
            &[donut_color(DonutColor::Dough), donut_color(DonutColor::Cocoa), donut_color(DonutColor::Cream)],
            1.0,
        ));
    })
}

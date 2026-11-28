//! The `bezel` face's drawing.

use super::color::BezelColor;
use super::color::bezel_color;
use crate::HOLE_RATIO;
use gtk4::DrawingArea;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillRect;
use smearor_commons_drawing::StrokePolyline;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Polyline;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
use std::f64::consts::PI;

/// `Clipped` face: same square extent as `Fill` but the `ShapeClip`
/// cuts it to the band - the frosting stays inside the ring, corners
/// and hole show card surface beneath.
pub(crate) fn build_bezel(size: Pixels) -> DrawingArea {
    drawing_target(Some(Size::new(f64::from(size) as i32, f64::from(size) as i32)), move |target| {
        let bounds = Rect::<Pixels>::from_size(target.size().to_f64().with_unit());
        let face = Circle::inscribed_in(&bounds);
        let _ = target.fill_rect(FillRect::new(bounds, bezel_color(BezelColor::Face), 1.0));
        // Tick ring: the clip leaves only what lands on the band.
        let tick = bezel_color(BezelColor::Ticks);
        for index in 0..36 {
            let radial = Vec2::<Pixels>::new(1.0, 0.0).rotated(RotationRad::new(index as f64 * 2.0 * PI / 36.0));
            let (inner_r, outer_r) = if index % 3 == 0 {
                (face.radius * HOLE_RATIO.as_f64(), face.radius)
            } else {
                (face.radius * 0.75, face.radius * 0.95)
            };
            let _ = target.stroke_polyline(StrokePolyline::new(
                Polyline::new(vec![face.center + radial * inner_r, face.center + radial * outer_r]),
                tick,
                Pixels::new(4.0),
                0.85,
            ));
        }
    })
}

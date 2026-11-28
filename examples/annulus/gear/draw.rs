//! The `gear` face's drawing.

use super::color::GearColor;
use super::color::gear_color;
use crate::HOLE_RATIO;
use gtk4::DrawingArea;
use gtk4::cairo::FillRule;
use smearor_commons_drawing::CairoTarget;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::StrokeCircle;
use smearor_commons_drawing::StrokePolyline;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Annulus;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Polyline;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;

/// `Cover` face: gear teeth and spokes spanning the whole bounding
/// box - the card's silhouette clip leaves only the toothed ring.
pub(crate) fn paint_gear(target: &mut CairoTarget, annulus: Annulus<Pixels>, phase: RotationRad) {
    let context = target.context().clone();
    let center = annulus.center;
    let (inner, outer) = (annulus.inner, annulus.outer);
    let teeth = 12;
    let pitch = RotationRad::FULL_TURN * (1.0 / f64::from(teeth));
    // Teeth: trapezoid ring around the rim.
    gear_color(GearColor::Teeth).apply(&context);
    context.new_path();
    for index in 0..teeth {
        for (t, r) in [(0.00, outer * 0.92), (0.15, outer), (0.45, outer), (0.60, outer * 0.92)] {
            let point = center + Vec2::<Pixels>::new(1.0, 0.0).rotated(phase + pitch * (f64::from(index) + t)) * r;
            context.line_to(point.x, point.y);
        }
    }
    context.close_path();
    context.set_fill_rule(FillRule::EvenOdd);
    context.arc(center.x, center.y, inner, 0.0, 2.0 * PI);
    context.fill().unwrap_or_default();
    // Spokes crossing the band.
    let spoke = gear_color(GearColor::Spoke);
    for index in 0..4 {
        let radial = Vec2::<Pixels>::new(1.0, 0.0).rotated(phase + RotationRad::new(index as f64 * PI / 2.0 + PI / 4.0));
        let _ = target.stroke_polyline(StrokePolyline::new(
            Polyline::new(vec![center + radial * inner, center + radial * (outer * 0.92)]),
            spoke,
            Pixels::new((outer * 0.09) as f32),
            1.0,
        ));
    }
    // Hub ring around the hole.
    let _ = target.stroke_circle(StrokeCircle::new(
        Circle::new(center, inner * 1.08),
        gear_color(GearColor::Hub),
        Pixels::new((outer * 0.05) as f32),
        1.0,
    ));
}

pub(crate) fn build_gear(size: Pixels, phase: Rc<Cell<f64>>) -> DrawingArea {
    drawing_target(Some(Size::new(f64::from(size) as i32, f64::from(size) as i32)), move |target| {
        let face = Circle::inscribed_in(&Rect::<Pixels>::from_size(target.size().to_f64().with_unit()));
        paint_gear(
            target,
            Annulus::new(face.center, face.radius * HOLE_RATIO.as_f64(), face.radius),
            RotationRad::new(phase.get()),
        );
    })
}

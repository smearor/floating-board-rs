//! The `compass` face's drawing.

use super::color::CompassColor;
use super::color::compass_color;
use gtk4::DrawingArea;
use smearor_commons_color::Color;
use smearor_commons_drawing::CairoTarget;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillCircle;
use smearor_commons_drawing::FillPolygon;
use smearor_commons_drawing::StrokeCircle;
use smearor_commons_drawing::StrokePolyline;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Polygon;
use smearor_commons_geometry::Polyline;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
use std::f64::consts::TAU;

/// A compass needle: a `length`-long triangle pointing `angle`
/// clockwise off the face's "up", plus a shorter tail on the opposite
/// side. `RotationRad::ZERO` points to the widget's top edge.
pub(crate) fn paint_needle(target: &mut CairoTarget, center: Vec2<Pixels>, angle: RotationRad, length: Pixels, half_width: Pixels, color: Color<f64>) {
    let direction = Vec2::<Pixels>::new(0.0, -1.0).rotated(angle);
    let perp = direction.perpendicular();
    for (tip, color) in [(length, color), (length * -0.5, compass_color(CompassColor::Line))] {
        let tip = center + direction * tip.as_f64();
        let left = center - perp * half_width.as_f64();
        let right = center + perp * half_width.as_f64();
        let _ = target.fill_polygon(FillPolygon::new(Polygon::new(vec![left, tip, right]), color, 1.0));
    }
}

/// Paints the compass face (dial + ticks), the green needle fixed to
/// the card's "up" and - when `world_up` is `Some` - the red needle,
/// pointing `world_up` so it keeps aiming at the board's top edge no
/// matter how the card is rotated.
pub(crate) fn paint_compass(target: &mut CairoTarget, size: Vec2<Pixels>, world_up: Option<RotationRad>) {
    let face = Circle::new(size * 0.5, size.x.min(size.y) / 2.0 - 3.0);
    let _ = target.fill_circle(FillCircle::new(face, compass_color(CompassColor::Face), 0.95));
    let _ = target.stroke_circle(StrokeCircle::new(face, compass_color(CompassColor::Line), Pixels::new(2.5), 1.0));
    let up = Vec2::<Pixels>::new(0.0, -1.0);
    for tick in 0..24 {
        let radial = up.rotated(RotationRad::new(f64::from(tick) * TAU / 24.0));
        let inner = face.radius - if tick % 6 == 0 { 14.0 } else { 7.0 };
        let tick_line = Polyline::new(vec![face.center + radial * inner, face.center + radial * (face.radius - 3.0)]);
        let _ = target.stroke_polyline(StrokePolyline::new(tick_line, compass_color(CompassColor::Line), Pixels::new(2.5), 1.0));
    }
    paint_needle(
        target,
        face.center,
        RotationRad::ZERO,
        Pixels::new(face.radius * 0.52),
        Pixels::new(5.0),
        compass_color(CompassColor::CardNeedle),
    );
    if let Some(angle) = world_up {
        paint_needle(
            target,
            face.center,
            angle,
            Pixels::new(face.radius * 0.62),
            Pixels::new(6.5),
            compass_color(CompassColor::WorldNeedle),
        );
    }
    let _ = target.fill_circle(FillCircle::new(Circle::new(face.center, 4.0), compass_color(CompassColor::Line), 1.0));
}

/// A compass disc - the face plus the green card-fixed needle. The
/// red world-fixed needle is installed by `main` once the card exists
/// (its draw func reads `card.rotation()`). `CardContent::Inscribed`
/// keeps the surrounding rim chrome.
pub(crate) fn build_compass(size: Pixels<i32>) -> DrawingArea {
    drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), |target| {
        paint_compass(target, target.size().to_f64().with_unit(), None);
    })
}

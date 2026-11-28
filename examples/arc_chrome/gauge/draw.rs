//! The `gauge` face's drawing.

use super::color::GaugeColor;
use super::color::gauge_color;
use gtk4::DrawingArea;
use gtk4::prelude::*;
use smearor_commons_drawing::Cap;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillCircle;
use smearor_commons_drawing::StrokeArc;
use smearor_commons_drawing::StrokeCircle;
use smearor_commons_drawing::StrokePolyline;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Arc;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Polyline;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::f64::consts::FRAC_PI_2;
use std::f64::consts::TAU;
use std::rc::Rc;

/// A gauge dial painted edge-to-edge for `CardContent::Cover`: the
/// level ring rides the very rim of the disc, so only the card's
/// silhouette clip bounds it - no ring stays card chrome. `level`
/// (0.0..=1.0) is shared state, so the satellites can repaint the
/// dial via `queue_draw`.
pub(crate) fn build_gauge(size: Pixels<i32>, level: Rc<Cell<Fraction>>) -> DrawingArea {
    let gauge = drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), move |target| {
        let bounds = Rect::<Pixels>::from_size(target.size().to_f64().with_unit());
        let face = Circle::inscribed_in(&bounds);
        let _ = target.fill_circle(FillCircle::new(face, gauge_color(GaugeColor::Face), 1.0));
        // Level ring on the rim: dim track + bright fill from the top.
        let track = Circle::new(face.center, face.radius - 9.0);
        let _ = target.stroke_circle(StrokeCircle::new(track, gauge_color(GaugeColor::Track), Pixels::new(9.0), 0.35));
        let value = level.get().clamped();
        let level_arc = Arc::new(RotationRad::new(-FRAC_PI_2), RotationRad::new(value.as_f64() * TAU));
        let _ = target.stroke_arc(StrokeArc::new(track, level_arc, gauge_color(GaugeColor::Level), Pixels::new(9.0), 1.0).with_cap(Cap::Round));
        // Tick ring inside the level ring.
        let up = Vec2::<Pixels>::new(0.0, -1.0);
        for tick in 0..24 {
            let radial = up.rotated(RotationRad::new(f64::from(tick) * TAU / 24.0));
            let inner = track.radius - if tick % 6 == 0 { 16.0 } else { 9.0 };
            let tick_line = Polyline::new(vec![face.center + radial * inner, face.center + radial * (track.radius - 4.0)]);
            let _ = target.stroke_polyline(StrokePolyline::new(tick_line, gauge_color(GaugeColor::Ticks), Pixels::new(1.5), 0.55));
        }
        // Needle pointing at the level on the dial.
        let needle = Polyline::new(vec![face.center, face.center + up.rotated(level_arc.end()) * -(face.radius - 30.0)]);
        let _ = target.stroke_polyline(StrokePolyline::new(needle, gauge_color(GaugeColor::Ink), Pixels::new(4.0), 1.0));
        let _ = target.fill_circle(FillCircle::new(Circle::new(face.center, 5.0), gauge_color(GaugeColor::Ink), 1.0));
    });
    gauge.set_widget_name("gauge-content");
    gauge
}

//! The `clock` face's drawing.

use super::color::ClockColor;
use super::color::clock_color;
use glib::ControlFlow;
use glib::DateTime;
use glib::timeout_add_local;
use gtk4::DrawingArea;
use gtk4::prelude::*;
use smearor_commons_drawing::Cap;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillCircle;
use smearor_commons_drawing::StrokeCircle;
use smearor_commons_drawing::StrokePolyline;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Polyline;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
use std::f64::consts::TAU;
use std::time::Duration;

/// A `DrawingArea` rendering an analog clock face, ticking once per
/// second. Two instances exist: the dial's content and its
/// `folded_content` (a widget has exactly one parent).
pub(crate) fn build_clock(size: Pixels<i32>) -> DrawingArea {
    let clock = drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), |target| {
        let bounds = Rect::<Pixels>::from_size(target.size().to_f64().with_unit());
        let face = Circle::inscribed_in(&bounds.inset(Pixels::new(3.0)));
        let _ = target.fill_circle(FillCircle::new(face, clock_color(ClockColor::Face), 0.92));
        let _ = target.stroke_circle(StrokeCircle::new(face, clock_color(ClockColor::Ink), Pixels::new(2.5), 1.0));
        // The `12 o'clock` direction: `(sin a, -cos a)` pointing up,
        // rotating clockwise in the y-down plane.
        let up = Vec2::<Pixels>::new(0.0, -1.0);
        for tick in 0..12 {
            let radial = up.rotated(RotationRad::new(f64::from(tick) * TAU / 12.0));
            let inner = face.radius - if tick % 3 == 0 { 11.0 } else { 6.0 };
            let tick_line = Polyline::new(vec![face.center + radial * inner, face.center + radial * (face.radius - 3.0)]);
            let _ = target.stroke_polyline(StrokePolyline::new(tick_line, clock_color(ClockColor::Ink), Pixels::new(2.5), 1.0));
        }
        let Ok(now) = DateTime::now_local() else { return };
        let seconds = now.seconds();
        let hands = [
            (
                ((now.hour() % 12) as f64 + (now.minute() as f64 + seconds / 60.0) / 60.0) * TAU / 12.0,
                face.radius * 0.5,
                5.0,
                clock_color(ClockColor::Ink),
            ),
            ((now.minute() as f64 + seconds / 60.0) * TAU / 60.0, face.radius * 0.72, 3.5, clock_color(ClockColor::Ink)),
            (seconds * TAU / 60.0, face.radius * 0.82, 1.5, clock_color(ClockColor::Accent)),
        ];
        for (angle, length, line_width, color) in hands {
            let hand = Polyline::new(vec![face.center, face.center + up.rotated(RotationRad::new(angle)) * length]);
            let _ = target.stroke_polyline(StrokePolyline::new(hand, color, Pixels::new(line_width), 1.0).with_cap(Cap::Round));
        }
        let _ = target.fill_circle(FillCircle::new(Circle::new(face.center, 3.5), clock_color(ClockColor::Accent), 1.0));
    });
    timeout_add_local(Duration::from_secs(1), {
        let clock = clock.clone();
        move || {
            clock.queue_draw();
            ControlFlow::Continue
        }
    });
    clock
}

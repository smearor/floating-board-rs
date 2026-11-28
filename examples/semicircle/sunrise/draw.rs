//! The `sunrise` face's drawing.

use super::color::SunriseColor;
use super::color::sunrise_color;
use gtk4::DrawingArea;
use smearor_commons_drawing::Cap;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillSector;
use smearor_commons_drawing::StrokePolyline;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Annulus;
use smearor_commons_geometry::Arc;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Polyline;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
use std::f64::consts::FRAC_PI_2;
use std::f64::consts::PI;
/// A `DrawingArea` rendering a sunrise for the awning card - a
/// bottom-aligned square (`CardContent::Inscribed` on a dome): a
/// sun rising over a horizon line with three rays above.
pub(crate) fn build_sunrise(size: Pixels<i32>) -> DrawingArea {
    drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), |target| {
        let size = target.size().to_f64().with_unit::<Pixels>();
        // The horizon sits on the dome's flat edge: the sun rises
        // over the card's bottom line.
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y - 4.0);
        let radius = Pixels::new(size.x.min(size.y) * 0.26);
        // The sun: a half-disc (zero inner radius pie slice) peeking
        // over the horizon.
        let sun = sunrise_color(SunriseColor::Sun);
        let disc = Annulus::<Pixels>::new(center, 0.0, radius.as_f64());
        let half = Arc::new(RotationRad::new(PI), RotationRad::new(PI));
        let _ = target.fill_sector(FillSector::new(disc, half, sun, 0.95));
        // Three rays fanning above the sun.
        for ray in [-0.6, 0.0, 0.6] {
            let direction = Vec2::<Pixels>::new(1.0, 0.0).rotated(RotationRad::new(-FRAC_PI_2 + ray));
            let _ = target.stroke_polyline(
                StrokePolyline::new(
                    Polyline::new(vec![
                        center + direction * (radius + Pixels::new(8.0)).as_f64(),
                        center + direction * (radius + Pixels::new(22.0)).as_f64(),
                    ]),
                    sun,
                    Pixels::new(4.0),
                    0.85,
                )
                .with_cap(Cap::Round),
            );
        }
        // The horizon line.
        let _ = target.stroke_polyline(StrokePolyline::new(
            Polyline::new(vec![Vec2::new(size.x * 0.08, center.y), Vec2::new(size.x * 0.92, center.y)]),
            sunrise_color(SunriseColor::Horizon),
            Pixels::new(3.0),
            0.9,
        ));
    })
}

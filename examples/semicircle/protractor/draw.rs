//! The `protractor` face's drawing.

use super::color::ProtractorColor;
use super::color::protractor_color;
use gtk4::DrawingArea;
use gtk4::prelude::*;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillCircle;
use smearor_commons_drawing::FillPolygon;
use smearor_commons_drawing::FillSector;
use smearor_commons_drawing::StrokeArc;
use smearor_commons_drawing::StrokePolyline;
use smearor_commons_drawing::StrokeSector;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Annulus;
use smearor_commons_geometry::Arc;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Polygon;
use smearor_commons_geometry::Polyline;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::f64::consts::PI;
use std::rc::Rc;
/// A `DrawingArea` rendering a protractor for `CardContent::Cover`:
/// the acrylic face and its degree ticks paint edge-to-edge - only
/// the card's silhouette clip bounds them, no ring stays chrome.
/// `reading` (radians from the flat edge's right end, 0..PI) is
/// shared state: the swing arm follows it, so a card-level click
/// gesture can repaint via `queue_draw`.
pub(crate) fn build_protractor(width: Pixels<i32>, reading: Rc<Cell<f64>>) -> DrawingArea {
    let protractor = drawing_target(Some(Size::new(width.as_raw(), width.as_raw() / 2)), move |target| {
        let size = target.size().to_f64().with_unit::<Pixels>();
        let radius = Pixels::new((size.x / 2.0).min(size.y));
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y);
        let bronze = protractor_color(ProtractorColor::Bronze);
        // The acrylic face: a half-disc edge-to-edge - a pie slice is
        // a sector with a zero inner radius; `half` sweeps PI -> 2PI.
        let dome = Annulus::<Pixels>::new(center, 0.0, radius.as_f64());
        let half = Arc::new(RotationRad::new(PI), RotationRad::new(PI));
        let _ = target.fill_sector(FillSector::new(dome, half, protractor_color(ProtractorColor::Face), 0.95));
        let _ = target.stroke_sector(StrokeSector::new(dome, half, bronze, Pixels::new(2.0), 0.9));
        // Degree ticks on the rim: one per 10 deg, long at 30 deg.
        for tick in 0..=18 {
            let angle = RotationRad::HALF_TURN - RotationRad::HALF_TURN * Fraction::new(f64::from(tick) / 18.0);
            let direction = Vec2::<Pixels>::new(1.0, 0.0).rotated(-angle);
            let inner = radius - Pixels::new(if tick % 3 == 0 { 18.0 } else { 9.0 });
            let _ = target.stroke_polyline(StrokePolyline::new(
                Polyline::new(vec![center + direction * inner.as_f64(), center + direction * (radius - Pixels::new(3.0)).as_f64()]),
                bronze,
                Pixels::new(2.0),
                0.9,
            ));
        }
        // The inner scale arc and the classic half-disc cutout on the
        // flat edge give it the protractor look.
        let _ = target.stroke_arc(StrokeArc::new(Circle::new(center, (radius * 0.62).as_f64()), half, bronze, Pixels::new(2.0), 0.9));
        let cutout = Annulus::<Pixels>::new(center, 0.0, (radius * 0.30).as_f64());
        let _ = target.fill_sector(FillSector::new(cutout, half, protractor_color(ProtractorColor::Cutout), 0.92));
        // The origin mark on the flat edge.
        let pivot = center - Vec2::<Pixels>::new(0.0, 4.0);
        let _ = target.fill_circle(FillCircle::new(Circle::new(pivot, 3.0), bronze, 0.9));
        // The swing arm: a tapered needle reading `reading` degrees
        // over the flat edge, pivoting on the origin mark.
        let direction = Vec2::<Pixels>::new(1.0, 0.0).rotated(-RotationRad::new(reading.get()));
        let perpendicular = direction.perpendicular();
        let tip = pivot + direction * (radius * 0.88).as_f64();
        let needle = protractor_color(ProtractorColor::Needle);
        let _ = target.fill_polygon(FillPolygon::new(
            Polygon::new(vec![tip, pivot + perpendicular * 8.0, pivot - perpendicular * 8.0]),
            needle,
            0.92,
        ));
        let _ = target.fill_circle(FillCircle::new(Circle::new(pivot, 5.0), needle, 0.92));
    });
    protractor.set_widget_name("protractor-content");
    protractor
}

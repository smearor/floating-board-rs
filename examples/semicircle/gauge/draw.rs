//! The `gauge` face's drawing.

use super::color::DomeColor;
use super::color::dome_color;
use gtk4::DrawingArea;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillCircle;
use smearor_commons_drawing::FillPolygon;
use smearor_commons_drawing::FillSector;
use smearor_commons_drawing::StrokePolyline;
use smearor_commons_drawing::StrokeSector;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Annulus;
use smearor_commons_geometry::Arc;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Polygon;
use smearor_commons_geometry::Polyline;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Semicircle;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
use std::f64::consts::PI;
/// A `DrawingArea` rendering a speedometer face sized to the dome's
/// bounding box (`2r x r`): the dial's virtual circle centers on the
/// box's bottom middle, ticks fan across the rim, and the needle
/// holds a fixed reading at mid-scale.
pub(crate) fn build_gauge(width: Pixels<i32>) -> DrawingArea {
    drawing_target(Some(Size::new(width.as_raw(), width.as_raw() / 2)), |target| {
        let size = target.size().to_f64().with_unit::<Pixels>();
        // The dial's virtual circle centers on the box's bottom middle.
        let dome = Semicircle::inscribed_in(&Rect::<Pixels>::from_size(size).inset(Pixels::new(4.0)));
        let radius = Pixels::new(dome.radius);
        let center = dome.center;
        // The dome face: a pie slice (zero inner radius) from the left
        // end over the top to the right end; `half` sweeps PI -> 2PI.
        let disc = Annulus::<Pixels>::new(center, 0.0, dome.radius);
        let half = Arc::new(RotationRad::new(PI), RotationRad::new(PI));
        let _ = target.fill_sector(FillSector::new(disc, half, dome_color(DomeColor::Face), 0.95));
        let _ = target.stroke_sector(StrokeSector::new(disc, half, dome_color(DomeColor::Line), Pixels::new(2.5), 1.0));
        // Tick marks fan across the rim: 13 ticks, long ones at the
        // quarter marks.
        let rim = radius - Pixels::new(4.0);
        for tick in 0..=12 {
            let angle = RotationRad::HALF_TURN - RotationRad::TWELFTH_TURN * f64::from(tick);
            let direction = Vec2::<Pixels>::new(1.0, 0.0).rotated(-angle);
            let inner = radius - Pixels::new(if tick % 3 == 0 { 16.0 } else { 9.0 });
            let _ = target.stroke_polyline(StrokePolyline::new(
                Polyline::new(vec![center + direction * inner.as_f64(), center + direction * rim.as_f64()]),
                dome_color(DomeColor::Line),
                Pixels::new(2.5),
                1.0,
            ));
        }
        // The needle: a tapered pointer at a fixed mid-scale reading.
        // No tail - the pivot sits on the flat edge, a tail would dip
        // below the widget's bottom and clip.
        let direction = Vec2::<Pixels>::new(1.0, 0.0).rotated(-RotationRad::SIXTH_TURN);
        let perpendicular = direction.perpendicular();
        let tip = center + direction * (radius * 0.72).as_f64();
        let _ = target.fill_polygon(FillPolygon::new(
            Polygon::new(vec![tip, center + perpendicular * 6.0, center - perpendicular * 6.0]),
            dome_color(DomeColor::Needle),
            1.0,
        ));
        let _ = target.fill_circle(FillCircle::new(Circle::new(center, 3.0), dome_color(DomeColor::Line), 1.0));
    })
}

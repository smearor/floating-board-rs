//! The `hive` face's drawing.

use super::color::HiveColor;
use super::color::hive_color;
use gtk4::DrawingArea;
use gtk4::prelude::*;
use smearor_commons_color::Color;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillPolygon;
use smearor_commons_drawing::FillRect;
use smearor_commons_drawing::StrokePolygon;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Hexagon;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
/// A honeycomb patch: the center hexagon plus its six neighbors -
/// flat-top hexagons tile the plane with centers spaced `sqrt(3)*r`
/// horizontally and `1.5*r` vertically.
pub(crate) fn build_hive(size: Pixels<i32>) -> DrawingArea {
    let area = drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), move |target| {
        let size = target.size().to_f64().with_unit::<Pixels>();
        let center = Vec2::<Pixels>::new(size.x / 2.0, size.y / 2.0);
        let cell = size.x.min(size.y) / 5.5;
        // Neighbor centers for flat-top tiling: the cells above and
        // below share the flat edges (`sqrt(3) * r` apart), the four
        // diagonal cells share the slanted edges (`1.5r` over,
        // `sqrt(3) * r / 2` down) - edge-to-edge, no gaps.
        let neighbors: [Vec2<()>; 6] = [
            Vec2::new(0.0, -1.732),
            Vec2::new(0.0, 1.732),
            Vec2::new(1.5, -0.866),
            Vec2::new(1.5, 0.866),
            Vec2::new(-1.5, -0.866),
            Vec2::new(-1.5, 0.866),
        ];
        // Painted square background: under `CardContent::Clipped`
        // the corner triangles are cut away and the card face shows
        // through them (a `Fill` content would keep them painted).
        let _ = target.fill_rect(FillRect::new(Rect::<Pixels>::from_size(size), hive_color(HiveColor::Backdrop), 1.0));
        let honey = hive_color(HiveColor::Honey);
        for (index, offset) in neighbors.iter().enumerate() {
            let cell_polygon = Hexagon::flat_top(center + offset.with_unit::<Pixels>() * cell, cell).to_polygon();
            let _ = target.fill_polygon(FillPolygon::new(
                cell_polygon.clone(),
                Color::<f64>::new(0.32 + 0.06 * f64::from(index as i32 % 3), 0.24, 0.08),
                1.0,
            ));
            let _ = target.stroke_polygon(StrokePolygon::new(cell_polygon, honey, Pixels::new(2.5), 1.0));
        }
        let center_polygon = Hexagon::flat_top(center, cell).to_polygon();
        let _ = target.fill_polygon(FillPolygon::new(center_polygon.clone(), hive_color(HiveColor::Center), 1.0));
        let _ = target.stroke_polygon(StrokePolygon::new(center_polygon, honey, Pixels::new(3.0), 1.0));
    });
    area.set_widget_name("hive-content");
    area
}

//! The `gem` face's drawing.

use super::color::GemColor;
use super::color::gem_color;
use gtk4::DrawingArea;
use gtk4::prelude::*;
use smearor_commons_color::Color;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillPolygon;
use smearor_commons_drawing::StrokePolygon;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Hexagon;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Polygon;
use smearor_commons_geometry::PolygonOrientation;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Size;
/// A brilliant-cut hexagonal gem: an inner "table" facet plus the
/// six crown facets between table edges and silhouette - `Cover`
/// content fills the card edge-to-edge.
pub(crate) fn build_gem(size: Pixels<i32>) -> DrawingArea {
    let area = drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), move |target| {
        let size = target.size().to_f64().with_unit::<Pixels>();
        // `CardContent::Cover` allocates the card's whole bounding
        // box `2R x sqrt(3)R` - the gem reaches the silhouette.
        let gem = Hexagon::inscribed_in(&Rect::<Pixels>::from_size(size), PolygonOrientation::FlatTop);
        let center = gem.center;
        let table = gem.circumradius * 0.55;
        let outer = gem.vertices();
        let inner = Hexagon::flat_top(gem.center, table).vertices();
        // Crown facets: alternating shade quads between inner edges
        // and the outer silhouette.
        for facet in 0..6 {
            let next = (facet + 1) % 6;
            let shade = 0.42 + 0.10 * f64::from(facet as i32 % 3);
            let crown = gem_color(GemColor::Crown);
            let quad = Polygon::new(vec![inner[facet], inner[next], outer[next], outer[facet]]);
            let _ = target.fill_polygon(FillPolygon::new(quad.clone(), Color::<f64>::new(crown.r * shade, crown.g * shade, crown.b * shade), 1.0));
            let _ = target.stroke_polygon(StrokePolygon::new(quad, gem_color(GemColor::CrownRim), Pixels::new(1.5), 1.0));
        }
        // The table: a light flat-top facet.
        let table_polygon = Hexagon::flat_top(center, table).to_polygon();
        let _ = target.fill_polygon(FillPolygon::new(table_polygon.clone(), gem_color(GemColor::Table), 1.0));
        let _ = target.stroke_polygon(StrokePolygon::new(table_polygon, gem_color(GemColor::TableRim), Pixels::new(2.0), 1.0));
    });
    area.set_widget_name("gem-content");
    area
}

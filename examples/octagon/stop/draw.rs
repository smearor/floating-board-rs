//! The `stop` face's drawing.

use super::color::StopColor;
use super::color::stop_color;
use gtk4::DrawingArea;
use gtk4::cairo::FontSlant;
use gtk4::cairo::FontWeight;
use gtk4::prelude::*;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillPolygon;
use smearor_commons_drawing::StrokePolygon;
use smearor_commons_drawing::Text;
use smearor_commons_drawing::ToyFont;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Octagon;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::PolygonOrientation;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Size;
/// A stop sign: red face, white inner ring, bold lettering - the
/// look everyone reads as an octagon.
pub(crate) fn build_stop(size: Pixels<i32>) -> DrawingArea {
    let area = drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), move |target| {
        let size = target.size().to_f64().with_unit::<Pixels>();
        // `CardContent::Cover` gives the widget the card's whole
        // bounding square - the sign reaches the silhouette
        // edge-to-edge.
        let sign = Octagon::inscribed_in(&Rect::<Pixels>::from_size(size), PolygonOrientation::FlatTop);
        let center = sign.center;
        let inradius = sign.inradius();
        // Red face with the white inner octagon ring.
        let ring = Octagon::with_inradius(center, inradius * 0.86, PolygonOrientation::FlatTop);
        let white = stop_color(StopColor::Marking);
        let _ = target.fill_polygon(FillPolygon::new(sign.to_polygon(), stop_color(StopColor::Face), 1.0));
        let _ = target.stroke_polygon(StrokePolygon::new(sign.to_polygon(), white, Pixels::new(3.0), 1.0));
        let _ = target.stroke_polygon(StrokePolygon::new(ring.to_polygon(), white, Pixels::new(3.0), 1.0));
        // The lettering fills the face.
        let font = ToyFont::new("Sans", Pixels::new((inradius * 0.52) as f32), FontSlant::Normal, FontWeight::Bold);
        let _ = target.show_rotated_text(Text::new("STOP", font, center, Radians::ZERO, white, 1.0));
    });
    area.set_widget_name("stop-content");
    area
}

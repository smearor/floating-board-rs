//! The `compass` face's drawing.

use super::color::CompassColor;
use super::color::compass_color;
use gtk4::DrawingArea;
use gtk4::cairo::FontSlant;
use gtk4::cairo::FontWeight;
use gtk4::prelude::*;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillCircle;
use smearor_commons_drawing::FillPolygon;
use smearor_commons_drawing::FillRect;
use smearor_commons_drawing::StrokePolygon;
use smearor_commons_drawing::Text;
use smearor_commons_drawing::ToyFont;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Octagon;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Polygon;
use smearor_commons_geometry::PolygonOrientation;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
/// An eight-point compass rose: cardinal arms reach the flat-edge
/// midpoints, intercardinal arms the diagonal-edge midpoints - the
/// octagon's own eightfold grid.
pub(crate) fn build_compass(size: Pixels<i32>) -> DrawingArea {
    let area = drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), move |target| {
        let size = target.size().to_f64().with_unit::<Pixels>();
        let stop = Octagon::inscribed_in(&Rect::<Pixels>::from_size(size), PolygonOrientation::FlatTop);
        let center = stop.center;
        let inradius = stop.inradius();
        // Painted square background: under `CardContent::Clipped`
        // the corner triangles are cut away and the card face shows
        // through them (a `Fill` content would keep them painted).
        let _ = target.fill_rect(FillRect::new(Rect::<Pixels>::from_size(size), compass_color(CompassColor::Backdrop), 1.0));
        // Night-blue face.
        let _ = target.fill_polygon(FillPolygon::new(stop.to_polygon(), compass_color(CompassColor::Face), 1.0));
        let _ = target.stroke_polygon(StrokePolygon::new(stop.to_polygon(), compass_color(CompassColor::Rim), Pixels::new(2.5), 1.0));
        // Eight arms: each arm is a slim kite from the center to an
        // edge midpoint - cardinals (flat edges, k even) full length,
        // intercardinals (diagonal edges) shorter.
        let cardinal_color = compass_color(CompassColor::Cardinal);
        let inter_color = compass_color(CompassColor::Intercardinal);
        for arm in 0..8 {
            let angle = RotationRad::new(f64::from(arm) * std::f64::consts::FRAC_PI_4 - std::f64::consts::FRAC_PI_2);
            let cardinal = arm % 2 == 0;
            let tip = inradius * if cardinal { 0.92 } else { 0.62 };
            let wing = inradius * if cardinal { 0.16 } else { 0.11 };
            let radial = Vec2::<Pixels>::new(1.0, 0.0).rotated(angle);
            let wing_radial = Vec2::<Pixels>::new(1.0, 0.0).rotated(angle + RotationRad::new(std::f64::consts::FRAC_PI_4));
            let kite = Polygon::new(vec![center, center + radial * tip, center + wing_radial * wing]);
            let _ = target.fill_polygon(FillPolygon::new(kite, if cardinal { cardinal_color } else { inter_color }, 1.0));
        }
        // Center hub + north label.
        let _ = target.fill_circle(FillCircle::new(Circle::new(center, inradius * 0.09), compass_color(CompassColor::Hub), 1.0));
        let font = ToyFont::new("Sans", Pixels::new((inradius * 0.24) as f32), FontSlant::Normal, FontWeight::Bold);
        let _ = target.show_rotated_text(Text::new(
            "N",
            font,
            Vec2::<Pixels>::new(center.x, center.y - inradius * 0.36),
            Radians::ZERO,
            compass_color(CompassColor::Hub),
            1.0,
        ));
    });
    area.set_widget_name("compass-content");
    area
}

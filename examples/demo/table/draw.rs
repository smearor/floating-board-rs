//! The `table` face's drawing - surface, grid, hint and user markers.

use super::color::TableColor;
use super::color::table_color;
use super::seats::MARKER_RADIUS;
use super::seats::marker_position;
use super::users::DemoUser;
use gtk4::DrawingArea;
use gtk4::cairo::FontSlant;
use gtk4::cairo::FontWeight;
use smearor_commons_color::Color;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillCircle;
use smearor_commons_drawing::FillRect;
use smearor_commons_drawing::StrokePolyline;
use smearor_commons_drawing::Text;
use smearor_commons_drawing::ToyFont;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Polyline;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Vec2;
use std::cell::RefCell;
use std::rc::Rc;

pub(crate) fn build_table_face(users: Rc<RefCell<Vec<DemoUser>>>) -> DrawingArea {
    drawing_target(None, move |target| {
        let size = target.size().to_f64().with_unit::<Pixels>();
        let bounds = Rect::<Pixels>::from_size(size);
        let _ = target.fill_rect(FillRect::new(bounds, table_color(TableColor::Surface), 1.0));
        // The 80px grid: verticals and horizontals as two-point polylines.
        let grid = table_color(TableColor::Grid);
        let mut x = 0.0;
        while x < size.x {
            let _ = target.stroke_polyline(StrokePolyline::new(Polyline::new(vec![Vec2::new(x, 0.0), Vec2::new(x, size.y)]), grid, Pixels::new(2.0), 1.0));
            x += 80.0;
        }
        let mut y = 0.0;
        while y < size.y {
            let _ = target.stroke_polyline(StrokePolyline::new(Polyline::new(vec![Vec2::new(0.0, y), Vec2::new(size.x, y)]), grid, Pixels::new(2.0), 1.0));
            y += 80.0;
        }

        // Gesture hint, dimmed as part of the background grid.
        let hint_font = ToyFont::new("Sans", Pixels::new(13.0), FontSlant::Normal, FontWeight::Normal);
        let hint = "Drag card header | 2+ finger move/rotate/stretch | click raises";
        let _ = target.show_rotated_text(Text::new(hint, hint_font, bounds.center(), Radians::ZERO, table_color(TableColor::Ink), 0.35));

        // Table users at their current seats (the roster drives both
        // the badges and "Pass card"). In seat mode the markers are
        // draggable along the table edge.
        let letter_font = ToyFont::new("Sans", Pixels::new(12.0), FontSlant::Normal, FontWeight::Bold);
        for (seat, letter, color) in users.borrow().iter() {
            let at = marker_position(*seat, size);
            let _ = target.fill_circle(FillCircle::new(Circle::new(at, MARKER_RADIUS.as_f64()), Color::<f64>::from(*color), 1.0));
            let _ = target.show_rotated_text(Text::new(*letter, letter_font.clone(), at, Radians::ZERO, table_color(TableColor::Ink), 1.0));
        }
    })
}

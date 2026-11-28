//! The `die` face's drawing.

use super::color::DieColor;
use super::color::die_color;
use gtk4::DrawingArea;
use gtk4::prelude::*;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillCircle;
use smearor_commons_drawing::FillPolygon;
use smearor_commons_drawing::FillRect;
use smearor_commons_drawing::StrokePolygon;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Hexagon;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::PolygonOrientation;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
/// A die face: `pips` pips in the standard grid pattern on a dark
/// face - content stays a square, matching the hexagon's inscribed
/// square.
pub(crate) fn build_die(size: Pixels<i32>, pips: u8) -> DrawingArea {
    let area = drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), move |target| {
        let size = target.size().to_f64().with_unit::<Pixels>();
        let die = Hexagon::inscribed_in(&Rect::<Pixels>::from_size(size).inset(Pixels::new(4.0)), PolygonOrientation::FlatTop);
        let center = die.center;
        let half = die.circumradius;
        // `CardContent::Fill` allocates the rim-shrunk silhouette's
        // bounding box: the painted square background reaches into
        // the corner triangles a `Clipped` content would lose.
        let _ = target.fill_rect(FillRect::new(Rect::<Pixels>::from_size(size), die_color(DieColor::Backdrop), 1.0));
        // The face: a flat-top hexagon keeps the die inside the
        // card's silhouette.
        let _ = target.fill_polygon(FillPolygon::new(die.to_polygon(), die_color(DieColor::Face), 1.0));
        let _ = target.stroke_polygon(StrokePolygon::new(die.to_polygon(), die_color(DieColor::Rim), Pixels::new(3.0), 1.0));
        // Standard pip layout on a 3x3 grid.
        let grid = half * 0.52;
        let pip_radius = half * 0.14;
        let spots: &[Vec2<()>] = match pips {
            1 => &[Vec2::new(0.0, 0.0)],
            2 => &[Vec2::new(-1.0, -1.0), Vec2::new(1.0, 1.0)],
            3 => &[Vec2::new(-1.0, -1.0), Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0)],
            4 => &[Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0), Vec2::new(-1.0, 1.0), Vec2::new(1.0, 1.0)],
            5 => &[
                Vec2::new(-1.0, -1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(-1.0, 1.0),
                Vec2::new(1.0, 1.0),
            ],
            _ => &[
                Vec2::new(-1.0, -1.0),
                Vec2::new(1.0, -1.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(-1.0, 1.0),
                Vec2::new(1.0, 1.0),
            ],
        };
        let pip = die_color(DieColor::Pip);
        for spot in spots {
            let _ = target.fill_circle(FillCircle::new(Circle::new(center + spot.with_unit::<Pixels>() * grid, pip_radius), pip, 1.0));
        }
    });
    area.set_widget_name("die-content");
    area
}

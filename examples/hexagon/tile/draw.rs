//! The `tile` face's drawing.

use super::color::TileColor;
use super::color::tile_color;
use gtk4::DrawingArea;
use gtk4::cairo::FontSlant;
use gtk4::cairo::FontWeight;
use gtk4::prelude::*;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillCircle;
use smearor_commons_drawing::FillPolygon;
use smearor_commons_drawing::StrokePolygon;
use smearor_commons_drawing::Text;
use smearor_commons_drawing::ToyFont;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Hexagon;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::PolygonOrientation;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
/// A game tile: a stroked flat-top hexagon on a sand-colored face
/// with a number token (value plus probability pips underneath) -
/// the look hexagon grids are known for.
pub(crate) fn build_tile(size: Pixels<i32>) -> DrawingArea {
    let area = drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), move |target| {
        let size = target.size().to_f64().with_unit::<Pixels>();
        let tile = Hexagon::inscribed_in(&Rect::<Pixels>::from_size(size).inset(Pixels::new(4.0)), PolygonOrientation::FlatTop);
        let center = tile.center;
        let radius = tile.circumradius;
        // The tile face.
        let _ = target.fill_polygon(FillPolygon::new(tile.to_polygon(), tile_color(TileColor::Face), 1.0));
        let _ = target.stroke_polygon(StrokePolygon::new(tile.to_polygon(), tile_color(TileColor::Rim), Pixels::new(4.0), 1.0));
        // The number token: a light disc, the value above, five pips
        // below (the "8" odds row).
        let token = radius * 0.42;
        let _ = target.fill_circle(FillCircle::new(Circle::new(center, token), tile_color(TileColor::Token), 1.0));
        let font = ToyFont::new("Sans", Pixels::new((token * 1.1) as f32), FontSlant::Normal, FontWeight::Bold);
        let _ = target.show_rotated_text(Text::new(
            "8",
            font,
            center - Vec2::<Pixels>::new(0.0, token * 0.15),
            Radians::ZERO,
            tile_color(TileColor::Ink),
            1.0,
        ));
        let pip = tile_color(TileColor::Ink);
        for pip_index in 0..5 {
            let offset = f64::from(pip_index - 2) * token * 0.28;
            let _ = target.fill_circle(FillCircle::new(Circle::new(center + Vec2::<Pixels>::new(offset, token * 0.55), token * 0.07), pip, 1.0));
        }
    });
    area.set_widget_name("tile-content");
    area
}

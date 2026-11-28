//! The `chip` face's drawing.

use super::color::ChipColor;
use super::color::chip_color;
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
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Octagon;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::PolygonOrientation;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::Size;
/// An octagonal casino plaque: the flat edges carry alternating
/// wedge marks (the chip edge inserts), the center shows the
/// denomination.
pub(crate) fn build_chip(size: Pixels<i32>) -> DrawingArea {
    let area = drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), move |target| {
        let size = target.size().to_f64().with_unit::<Pixels>();
        let chip = Octagon::inscribed_in(&Rect::<Pixels>::from_size(size).inset(Pixels::new(2.0)), PolygonOrientation::FlatTop);
        let center = chip.center;
        let inradius = chip.inradius();
        // Deep green felt face.
        let _ = target.fill_polygon(FillPolygon::new(chip.to_polygon(), chip_color(ChipColor::Felt), 1.0));
        let _ = target.stroke_polygon(StrokePolygon::new(chip.to_polygon(), chip_color(ChipColor::Rim), Pixels::new(3.0), 1.0));
        // The eight edge inserts: a light wedge on each edge midpoint,
        // rotated by `k * 45°` around the center - the plaques' own
        // eightfold mark.
        // A thin trapezoid insert per edge: outer side a touch
        // narrower (squeeze 0.7) and slightly inside the rim (0.97),
        // inner side reaching 0.66 of the inradius.
        let wedge_color = chip_color(ChipColor::Wedge);
        for wedge in chip.wedges(Fraction::new(0.97), Fraction::new(0.66), Fraction::new(0.7)) {
            let _ = target.fill_polygon(FillPolygon::new(wedge, wedge_color, 1.0));
        }
        // The denomination.
        let font = ToyFont::new("Sans", Pixels::new((inradius * 0.38) as f32), FontSlant::Normal, FontWeight::Bold);
        let _ = target.show_rotated_text(Text::new("500", font, center, Radians::ZERO, chip_color(ChipColor::Denomination), 1.0));
    });
    area.set_widget_name("chip-content");
    area
}

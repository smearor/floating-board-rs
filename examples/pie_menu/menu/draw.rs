//! The pie menu's drawing.

use super::color::MenuColor;
use super::color::menu_color;
use super::color::wedge_color;
use super::geometry::menu_annulus;
use super::geometry::wedge_arc;
use super::items::ITEMS;
use super::state::MenuState;
use gtk4::DrawingArea;
use gtk4::cairo::FontSlant;
use gtk4::cairo::FontWeight;
use gtk4::prelude::*;
use smearor_commons_drawing::CairoTarget;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillSector;
use smearor_commons_drawing::StrokeCircle;
use smearor_commons_drawing::StrokeSector;
use smearor_commons_drawing::Text;
use smearor_commons_drawing::ToyFont;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Radians;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::Vec2;
use std::f64::consts::PI;
use std::f64::consts::TAU;
use std::rc::Rc;

/// Paints the wedge ring: every item's sector fills the band, labels
/// ride the mid radius reading outward (flipped on the left half so
/// they never run upside-down). The hovered wedge lifts and gains a
/// light outline; the active one stays brightest until superseded.
pub(crate) fn paint_menu(target: &mut CairoTarget, size: Vec2<Pixels>, state: &MenuState) {
    let annulus = menu_annulus(size);
    let (inner, outer) = (annulus.inner, annulus.outer);
    let step = TAU / ITEMS.len() as f64;

    for (index, (label, slot)) in ITEMS.iter().enumerate() {
        let (lift, alpha) = match (state.hover.get() == Some(index), state.active.get() == Some(index)) {
            (true, _) => (0.30, 1.0),
            (false, true) => (0.22, 0.98),
            (false, false) => (0.0, 0.82),
        };
        let _ = target.fill_sector(FillSector::new(annulus, wedge_arc(index), wedge_color(*slot).brighten(lift).clamp(), alpha));
        if state.hover.get() == Some(index) {
            let _ = target.stroke_sector(StrokeSector::new(annulus, wedge_arc(index), menu_color(MenuColor::Ink), Pixels::new(2.5), 0.7));
        }
        // Label on the mid radius, reading outward; on the left half
        // the text flips so it stays upright.
        let mid = index as f64 * step + step / 2.0;
        let direction = RotationRad::new(mid - PI / 2.0);
        let flip = mid > PI / 2.0 && mid < 3.0 * PI / 2.0;
        let at = annulus.center + Vec2::<Pixels>::new(1.0, 0.0).rotated(direction) * (inner + (outer - inner) / 2.0);
        let angle = Radians::new(direction.as_f64() + if flip { PI } else { 0.0 });
        let font = ToyFont::new("Sans", Pixels::new(15.0), FontSlant::Normal, FontWeight::Bold);
        let _ = target.show_rotated_text(Text::new(*label, font, at, angle, menu_color(MenuColor::Ink), 0.95));
    }
    // Rim rings tighten the silhouette edges.
    let rim = menu_color(MenuColor::Rim);
    let _ = target.stroke_circle(StrokeCircle::new(Circle::new(annulus.center, outer - 1.0), rim, Pixels::new(2.0), 0.8));
    let _ = target.stroke_circle(StrokeCircle::new(Circle::new(annulus.center, inner + 1.0), rim, Pixels::new(2.0), 0.8));
}

/// The menu face: a `Cover` DrawingArea painting the wedge ring
/// edge-to-edge - the card's silhouette clip cuts it to the band.
pub(crate) fn build_menu_face(size: Pixels, state: Rc<MenuState>) -> DrawingArea {
    let face = drawing_target(Some(Size::new(f64::from(size) as i32, f64::from(size) as i32)), move |target| {
        paint_menu(target, target.size().to_f64().with_unit(), &state);
    });
    face.set_widget_name("pie-face");
    face
}

//! The `lifebuoy` face's drawing.

use super::color::LifebuoyColor;
use super::color::lifebuoy_color;
use gtk4::DrawingArea;
use gtk4::prelude::*;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillCircle;
use smearor_commons_drawing::FillSectors;
use smearor_commons_drawing::StrokeCircle;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Annulus;
use smearor_commons_geometry::Arc;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;

/// `Inscribed` face: a red/white lifebuoy ring centered in the square
/// allocation - the hole region is left unpainted where the band
/// leaves it, the square's corners cover the rest.
pub(crate) fn build_lifebuoy(size: Pixels) -> DrawingArea {
    let face = drawing_target(Some(Size::new(f64::from(size) as i32, f64::from(size) as i32)), move |target| {
        let bounds = Rect::<Pixels>::from_size(target.size().to_f64().with_unit());
        let face = Circle::inscribed_in(&bounds);
        // A faint disc shows the square's extent: `Inscribed`
        // paints over the hole region on purpose.
        let _ = target.fill_circle(FillCircle::new(face, lifebuoy_color(LifebuoyColor::Face), 0.35));
        let _ = target.fill_sectors(FillSectors::new(
            Annulus::new(face.center, face.radius * 0.95 * 0.55, face.radius),
            Arc::new(RotationRad::ZERO, RotationRad::FULL_TURN),
            8,
            RotationRad::ZERO,
            &[lifebuoy_color(LifebuoyColor::Red), lifebuoy_color(LifebuoyColor::White)],
            1.0,
        ));
        let _ = target.stroke_circle(StrokeCircle::new(face, lifebuoy_color(LifebuoyColor::Rim), Pixels::new(2.0), 0.9));
    });
    face.set_widget_name("lifebuoy-content");
    face
}

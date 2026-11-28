//! The `pie` face's drawing.

use super::color::pie_color;
use super::shares::PieShares;
use gtk4::DrawingArea;
use smearor_commons_drawing::DrawPrimitiveOps;
use smearor_commons_drawing::FillSector;
use smearor_commons_drawing::gtk::drawing_target;
use smearor_commons_geometry::Annulus;
use smearor_commons_geometry::Arc;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Rect;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::Size;
use std::cell::RefCell;
use std::f64::consts::FRAC_PI_2;
use std::f64::consts::TAU;
use std::rc::Rc;

/// A pie chart drawing `shares` - mounted as
/// `CardContent::Inscribed` so the surrounding ring stays chrome.
/// The draw func reads the shared state, so satellite clicks repaint
/// via `queue_draw`.
pub(crate) fn build_pie(size: Pixels<i32>, shares: Rc<RefCell<PieShares>>) -> DrawingArea {
    drawing_target(Some(Size::new(size.as_raw(), size.as_raw())), move |target| {
        let bounds = Rect::<Pixels>::from_size(target.size().to_f64().with_unit());
        let face = Circle::inscribed_in(&bounds.inset(Pixels::new(3.0)));
        // Pie slices are sectors down to the center - the `inner = 0`
        // annulus degenerates its back-arc to a point.
        let disc = Annulus::new(face.center, 0.0, face.radius);
        let mut angle = RotationRad::new(-FRAC_PI_2);
        for (slot, share) in shares.borrow().shares {
            if share <= Fraction::ZERO {
                continue;
            }
            let arc = Arc::new(angle, RotationRad::new(share.as_f64() * TAU));
            let _ = target.fill_sector(FillSector::new(disc, arc, pie_color(slot), 1.0));
            angle += arc.width;
        }
    })
}

//! The `pie` face's slice satellite.

use super::color::PieColor;
use super::shares::PieShares;
use super::shares::SLICE_FLOOR;
use super::shares::SLICE_STEP;
use gtk4::Button;
use gtk4::DrawingArea;
use gtk4::prelude::*;
use smearor_commons_geometry::Fraction;
use std::cell::RefCell;
use std::rc::Rc;

/// A slice satellite: a round dot in the slice's color that grows its
/// slice by `amount` on click - `Red`'s first click seeds a
/// `SLICE_FLOOR` share instead of stepping.
pub(crate) fn slice_satellite(class: &str, face: &DrawingArea, shares: &Rc<RefCell<PieShares>>, slot: PieColor) -> Button {
    let button = Button::new();
    button.add_css_class("pie-dot");
    button.add_css_class(class);
    let face = face.downgrade();
    let shares = shares.clone();
    button.connect_clicked(move |_| {
        let amount = if slot == PieColor::Red && shares.borrow().share(slot) <= Fraction::ZERO {
            // The first red click seeds a full floor share - a
            // `SLICE_STEP` seed would undercut it.
            SLICE_FLOOR
        } else {
            SLICE_STEP
        };
        shares.borrow_mut().grow(slot, amount);
        if let Some(face) = face.upgrade() {
            face.queue_draw();
        }
    });
    button
}

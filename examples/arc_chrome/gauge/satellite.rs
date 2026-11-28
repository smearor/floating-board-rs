//! The `gauge` face's level satellite.

use gtk4::Button;
use gtk4::DrawingArea;
use gtk4::prelude::*;
use smearor_commons_geometry::Fraction;
use std::cell::Cell;
use std::rc::Rc;

/// A level satellite: a round `+`/`-` button stepping the shared
/// `level` by `delta` on click - the gauge face repaints via
/// `queue_draw`. The cover content itself is display-only
/// (non-targetable), so satellites carry the interaction.
pub(crate) fn level_satellite(label: &str, class: &str, delta: f64, face: &DrawingArea, level: &Rc<Cell<Fraction>>) -> Button {
    let button = Button::with_label(label);
    button.add_css_class("gauge-btn");
    button.add_css_class(class);
    let face = face.downgrade();
    let level = level.clone();
    button.connect_clicked(move |_| {
        level.set((level.get() + Fraction::new(delta)).clamped());
        if let Some(face) = face.upgrade() {
            face.queue_draw();
        }
    });
    button
}

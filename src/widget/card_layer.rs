//! `CardLayer` - the transparent `gtk4::Fixed` subclass holding the
//! card transform stacks.

use gtk4::glib;

use crate::widget::imp::card_layer::CardLayerImpl;

glib::wrapper! {
    /// Card layer: a `gtk4::Fixed` that is never itself an event
    /// target - picks pass through to its card children or to the
    /// board's content below.
    pub struct CardLayer(ObjectSubclass<CardLayerImpl>)
        @extends gtk4::Fixed, gtk4::Widget,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget;
}

impl CardLayer {
    /// Creates the empty card layer.
    pub fn new() -> Self {
        glib::Object::builder().build()
    }
}

impl Default for CardLayer {
    fn default() -> Self {
        Self::new()
    }
}

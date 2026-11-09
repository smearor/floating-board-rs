//! `CardLayerImpl` - the `gtk4::Fixed` card layer that only lets its
//! children pick events.

use glib::object_subclass;
use gtk4::subclass::prelude::FixedImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::WidgetImpl;

use crate::widget::CardLayer;

/// Implementation of [`CardLayer`].
#[derive(Default)]
pub struct CardLayerImpl;

#[object_subclass]
impl ObjectSubclass for CardLayerImpl {
    const NAME: &'static str = "FloatingBoardCardLayer";
    type Type = CardLayer;
    type ParentType = gtk4::Fixed;
}

impl ObjectImpl for CardLayerImpl {}

impl WidgetImpl for CardLayerImpl {
    /// The layer itself is never an event target - only the cards
    /// placed on it are. GTK pick checks children *before* consulting
    /// `contains` for the parent itself, so reporting `false` here
    /// passes clicks on empty areas through to the content below
    /// while the cards stay interactive.
    fn contains(&self, _x: f64, _y: f64) -> bool {
        false
    }
}

impl FixedImpl for CardLayerImpl {}

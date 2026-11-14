//! `CardLayer` - the transparent `gtk4::Fixed` subclass holding the
//! card transform stacks.

use crate::widget::imp::card_layer::CardLayerImpl;
use gtk4::EventControllerLegacy;
use gtk4::GestureDrag;
use gtk4::gdk::Event;
use gtk4::glib;
use gtk4::graphene::Point;
use gtk4::prelude::EventControllerExt;
use gtk4::prelude::GestureDragExt;
use gtk4::prelude::WidgetExt;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;

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

    /// Pointer position of a drag gesture in layer coordinates, taken
    /// from the gesture's own point tracking (`start_point + offset`
    /// in its widget space, mapped through `compute_point`). The round
    /// trip through the transform stack yields the true layer-space
    /// position: GTK delivers the widget-space point via the inverse
    /// transform, so applying the forward transform returns the surface
    /// point - independent of the card's own motion, and always fresh
    /// at signal emission time (unlike `current_event`, which can lag
    /// one touch event behind).
    pub(crate) fn gesture_pointer(&self, gesture: &GestureDrag) -> Option<Vec2<Pixels>> {
        let start = Vec2::<Pixels>::from(gesture.start_point()?);
        let offset = Vec2::<Pixels>::from(gesture.offset()?);
        let widget = gesture.widget()?;
        let point = widget.compute_point(self, &Point::from((start + offset).to_f32()))?;
        Some(Vec2::<Pixels, f32>::from(point).to_f64())
    }

    /// Touch position of a raw event in layer coordinates: the event
    /// reports its position in surface space; it is mapped into the
    /// controller widget's coordinate space through the native's
    /// `compute_point` and then down the transform stack into layer
    /// (board) space.
    pub(crate) fn event_pointer(&self, controller: &EventControllerLegacy, event: &Event) -> Option<Vec2<Pixels>> {
        let position = Vec2::<Pixels>::from(event.position()?);
        let widget = controller.widget()?;
        let native = widget.native()?;
        let surface_point = Point::from(position.to_f32());
        let widget_point = native.compute_point(&widget, &surface_point)?;
        let point = widget.compute_point(self, &widget_point)?;
        Some(Vec2::<Pixels, f32>::from(point).to_f64())
    }
}

impl Default for CardLayer {
    fn default() -> Self {
        Self::new()
    }
}

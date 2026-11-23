use crate::widget::imp::circular_clip::CircularClipImpl;
use gtk4::Accessible;
use gtk4::Buildable;
use gtk4::ConstraintTarget;
use gtk4::Widget;
use gtk4::glib;
use gtk4::prelude::IsA;
use gtk4::prelude::WidgetExt;

glib::wrapper! {
    /// Single-child container that clips its content to the circle
    /// inscribed in its allocation - both in rendering and picking.
    ///
    /// Mounted for `CircleContent::Clipped`: crescent picks fall through
    /// to the card below, keeping the ring a drag affordance.
    pub struct CircularClip(ObjectSubclass<CircularClipImpl>)
        @extends Widget,
        @implements Accessible, Buildable, ConstraintTarget;
}

impl CircularClip {
    /// Creates an empty clip container.
    pub(crate) fn new() -> Self {
        glib::Object::builder().build()
    }

    /// Mounts `child` as the clipped content.
    pub(crate) fn set_child(&self, child: &impl IsA<Widget>) {
        child.set_parent(self);
    }
}

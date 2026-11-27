use crate::widget::imp::silhouette_box::SilhouetteBoxImpl;
use gtk4::Accessible;
use gtk4::Box;
use gtk4::Buildable;
use gtk4::ConstraintTarget;
use gtk4::Orientable;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::glib;

glib::wrapper! {
    /// A `gtk4::Box` whose pick region is bound by the ancestor card's
    /// silhouette instead of its own rectangular allocation: pick
    /// probes in the cut regions outside the shape report `false`
    /// (GTK consults `contains` only as a leaf check), so they pass
    /// through to the cards below instead of being swallowed.
    pub struct SilhouetteBox(ObjectSubclass<SilhouetteBoxImpl>)
        @extends Widget, Box,
        @implements Accessible, Buildable, ConstraintTarget, Orientable;
}

impl SilhouetteBox {
    /// Creates a box with `orientation` and `spacing` (like
    /// `gtk4::Box::new`).
    pub(crate) fn new(orientation: Orientation, spacing: i32) -> Self {
        glib::Object::builder()
            .property("orientation", orientation)
            .property("spacing", spacing)
            .build()
    }
}

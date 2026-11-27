use crate::domain::CardShape;
use crate::widget::imp::shape_clip::ShapeClipImpl;
use gtk4::Accessible;
use gtk4::Buildable;
use gtk4::ConstraintTarget;
use gtk4::Widget;
use gtk4::glib;
use gtk4::prelude::IsA;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

glib::wrapper! {
    /// Single-child container that clips its content to the card
    /// shape inscribed in its allocation - both in rendering and
    /// picking.
    ///
    /// Mounted for `CardContent::Clipped` (cut-region picks fall
    /// through to the card below, keeping the ring a drag
    /// affordance) and `CardContent::Cover` (the full-box content is
    /// cut to the silhouette).
    pub struct ShapeClip(ObjectSubclass<ShapeClipImpl>)
        @extends Widget,
        @implements Accessible, Buildable, ConstraintTarget;
}

impl ShapeClip {
    /// Creates an empty clip container cutting to `shape` - a
    /// band of `inner_ratio` of its outer radius on `Annulus`,
    /// ignored on every other shape.
    pub(crate) fn new(shape: CardShape, inner_ratio: f64) -> Self {
        let clip: Self = glib::Object::builder().build();
        clip.imp().shape.set(shape);
        clip.imp().inner_ratio.set(inner_ratio);
        clip
    }

    /// Mounts `child` as the clipped content.
    pub(crate) fn set_child(&self, child: &impl IsA<Widget>) {
        child.set_parent(self);
    }
}

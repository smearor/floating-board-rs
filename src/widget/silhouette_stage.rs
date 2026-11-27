use crate::widget::imp::silhouette_stage::SilhouetteStageImpl;
use crate::widget::imp::silhouette_stage::StageLayoutImpl;
use gtk4::Accessible;
use gtk4::Buildable;
use gtk4::ConstraintTarget;
use gtk4::LayoutManager;
use gtk4::Widget;
use gtk4::glib;
use gtk4::prelude::Cast;
use gtk4::prelude::IsA;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;

glib::wrapper! {
    /// Layout manager driving [`SilhouetteStage`]: the base child
    /// fills the extent, overlay children get their natural size at
    /// their own `halign`/`valign` - `gtk4::Overlay` semantics
    /// (`gtk4::Overlay` is not subclassable, so the stage replicates
    /// it).
    pub struct StageLayout(ObjectSubclass<StageLayoutImpl>)
        @extends LayoutManager;
}

glib::wrapper! {
    /// A `gtk4::Overlay`-style stack whose pick region is bound by
    /// the ancestor card's silhouette instead of its own rectangular
    /// allocation: pick probes in the cut regions outside the shape
    /// report `false` (GTK consults `contains` only as a leaf
    /// check), so they pass through to the cards below instead of
    /// being swallowed.
    pub struct SilhouetteStage(ObjectSubclass<SilhouetteStageImpl>)
        @extends Widget,
        @implements Accessible, Buildable, ConstraintTarget;
}

impl SilhouetteStage {
    /// Creates an empty stage.
    pub(crate) fn new() -> Self {
        glib::Object::builder().build()
    }

    /// Sets the base child filling the stage (like `Overlay::set_child`).
    pub(crate) fn set_child(&self, child: Option<&impl IsA<Widget>>) {
        if let Some(old) = self.imp().child.borrow_mut().take() {
            old.unparent();
        }
        if let Some(new_child) = child {
            // The base is the first child - overlay children follow.
            new_child.insert_before(self, self.first_child().as_ref());
            *self.imp().child.borrow_mut() = Some(new_child.clone().upcast());
        }
        self.queue_resize();
    }

    /// Stacks `widget` over the base child (like `Overlay::add_overlay`):
    /// it claims no layout space and is allocated its natural size at
    /// its own `halign`/`valign`.
    pub(crate) fn add_overlay(&self, widget: &impl IsA<Widget>) {
        widget.set_parent(self);
        self.queue_resize();
    }
}

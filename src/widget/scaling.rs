use crate::widget::imp::scaling::ScalingLayoutImpl;
use crate::widget::imp::scaling::ScalingWidgetImpl;
use gtk4::Accessible;
use gtk4::Buildable;
use gtk4::ConstraintTarget;
use gtk4::LayoutManager;
use gtk4::Widget;
use gtk4::prelude::*;
use gtk4::subclass::prelude::*;

glib::wrapper! {
    /// Layout manager driving [`ScalingWidget`]; exposes `scale` as a
    /// notify-able `f64` property.
    pub struct ScalingLayout(ObjectSubclass<ScalingLayoutImpl>)
        @extends LayoutManager;
}

glib::wrapper! {
    /// Container that renders its child scaled around the center.
    ///
    /// Outer element of the card transform stack:
    /// `ScalingWidget { RotationWidget { Card } }`.
    pub struct ScalingWidget(ObjectSubclass<ScalingWidgetImpl>)
        @extends Widget,
        @implements Accessible, Buildable, ConstraintTarget;
}

impl ScalingWidget {
    /// Creates a scaling container for `child` at scale `1.0`.
    pub fn new(child: &impl IsA<Widget>) -> Self {
        let widget: Self = glib::Object::builder().build();
        widget.set_child(Some(child));
        widget
    }

    /// Returns the scaling layout of this widget.
    pub fn scaling_layout(&self) -> Option<ScalingLayout> {
        self.layout_manager().and_then(|manager| manager.downcast::<ScalingLayout>().ok())
    }

    /// Current uniform scale factor (`1.0` = natural size).
    pub fn scale(&self) -> f64 {
        self.scaling_layout().map(|layout| layout.imp().scale.get()).unwrap_or(1.0)
    }

    /// Sets the scale factor and triggers a re-measure so the parent
    /// (`gtk4::Fixed`) re-allocates the resized footprint.
    pub fn set_scale(&self, scale: f64) {
        if let Some(layout) = self.scaling_layout() {
            layout.imp().scale.set(scale);
            layout.layout_changed();
        }
        self.queue_resize();
    }

    /// Returns the wrapped child widget.
    pub fn child(&self) -> Option<Widget> {
        self.imp().child.borrow().clone()
    }

    /// Replaces the wrapped child widget.
    pub fn set_child(&self, child: Option<&impl IsA<Widget>>) {
        let mut stored = self.imp().child.borrow_mut();
        if let Some(old_child) = stored.take() {
            old_child.unparent();
        }
        if let Some(new_child) = child {
            new_child.set_parent(self);
            *stored = Some(new_child.clone().upcast());
        }
        drop(stored);
        self.queue_resize();
    }
}

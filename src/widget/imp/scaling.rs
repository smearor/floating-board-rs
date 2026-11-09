use crate::widget::scaling::ScalingLayout;
use crate::widget::scaling::ScalingWidget;
use glib::subclass::prelude::ObjectImpl;
use glib::subclass::prelude::ObjectSubclass;
use gtk4::Orientation;
use gtk4::glib::Properties;
use gtk4::graphene::Point;
use gtk4::gsk::Transform;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::layout_manager::LayoutManagerImpl;
use gtk4::subclass::prelude::DerivedObjectProperties;
use gtk4::subclass::prelude::WidgetClassExt;
use gtk4::subclass::prelude::WidgetImpl;
use std::cell::Cell;
use std::cell::RefCell;

/// Widget implementation for [`ScalingWidget`].
///
/// The widget itself only tracks the child; the scale transform lives in
/// [`ScalingLayoutImpl`] as a notify-able property so observers (the board)
/// can react to scale changes.
#[derive(Default)]
pub struct ScalingWidgetImpl {
    /// The wrapped child (a `RotationWidget` in the card stack).
    pub child: RefCell<Option<gtk4::Widget>>,
}

#[glib::object_subclass]
impl ObjectSubclass for ScalingWidgetImpl {
    const NAME: &'static str = "FloatingBoardScalingWidget";
    type Type = ScalingWidget;
    type ParentType = gtk4::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<ScalingLayout>();
    }
}

impl ObjectImpl for ScalingWidgetImpl {
    fn dispose(&self) {
        if let Some(child) = self.child.borrow_mut().take() {
            child.unparent();
        }
    }
}

impl WidgetImpl for ScalingWidgetImpl {}

/// Layout manager that renders the child scaled by `scale` around its center.
///
/// `measure` reports `child_natural * scale` to the outside; `allocate` gives
/// the child its natural size and applies a `gsk::Transform::scale` around the
/// allocation center. Picking needs no extra work: GTK4 applies the inverse of
/// the allocation transform when descending into children, which - combined
/// with `RotationWidget`'s own transform - yields the full inverse stack
/// `scale^-1 . rotate^-1`.
#[derive(Properties)]
#[properties(wrapper_type = ScalingLayout)]
pub struct ScalingLayoutImpl {
    /// Uniform scale factor (`1.0` = natural size).
    #[properties(get, set, notify)]
    pub scale: Cell<f64>,
}

impl Default for ScalingLayoutImpl {
    fn default() -> Self {
        Self { scale: Cell::new(1.0) }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ScalingLayoutImpl {
    const NAME: &'static str = "FloatingBoardScalingLayout";
    type Type = ScalingLayout;
    type ParentType = gtk4::LayoutManager;
}

#[glib::derived_properties]
impl ObjectImpl for ScalingLayoutImpl {}

impl ScalingLayoutImpl {
    fn natural_child_size(&self, widget: &gtk4::Widget) -> (i32, i32) {
        let Some(child) = widget.first_child() else {
            return (0, 0);
        };
        let (_, natural_width, _, _) = child.measure(Orientation::Horizontal, -1);
        let (_, natural_height, _, _) = child.measure(Orientation::Vertical, -1);
        (natural_width, natural_height)
    }
}

impl LayoutManagerImpl for ScalingLayoutImpl {
    fn measure(&self, widget: &gtk4::Widget, orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        let scale = self.scale.get();
        if let Some(child) = widget.first_child() {
            let (minimum, natural, _, _) = child.measure(orientation, for_size);
            ((minimum as f64 * scale).ceil() as i32, (natural as f64 * scale).ceil() as i32, -1, -1)
        } else {
            (0, 0, -1, -1)
        }
    }

    fn allocate(&self, widget: &gtk4::Widget, width: i32, height: i32, baseline: i32) {
        let Some(child) = widget.first_child() else {
            return;
        };
        let scale = self.scale.get() as f32;
        let (natural_width, natural_height) = self.natural_child_size(widget);
        let child_width = if natural_width > 0 { natural_width } else { width };
        let child_height = if natural_height > 0 { natural_height } else { height };
        // Center the unscaled child in the allocation, then scale around the center.
        let transform = Transform::new()
            .translate(&Point::new(width as f32 / 2.0, height as f32 / 2.0))
            .scale(scale, scale)
            .translate(&Point::new(child_width as f32 / -2.0, child_height as f32 / -2.0));
        child.allocate(child_width, child_height, baseline, Some(transform));
    }
}

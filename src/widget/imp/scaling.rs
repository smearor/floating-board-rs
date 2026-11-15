use crate::widget::scaling::ScalingLayout;
use crate::widget::scaling::ScalingWidget;
use glib::subclass::prelude::ObjectImpl;
use glib::subclass::prelude::ObjectSubclass;
use glib::subclass::types::ObjectSubclassExt;
use gtk4::Orientation;
use gtk4::glib::Properties;
use gtk4::gsk::Transform;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::layout_manager::LayoutManagerImpl;
use gtk4::subclass::prelude::DerivedObjectProperties;
use gtk4::subclass::prelude::WidgetClassExt;
use gtk4::subclass::prelude::WidgetImpl;
use smearor_commons_geometry::CenteredTransform;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::ScaleFactor;
use smearor_commons_geometry::Size;
use smearor_commons_geometry::WidgetSizeExt;
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

impl WidgetImpl for ScalingWidgetImpl {
    /// Picking must test the *displayed* card, not this widget's
    /// allocation: the allocation is the rotated card's bounding box,
    /// so without the override a rotated card claims input in its
    /// invisible corner areas and starves the cards behind it.
    /// `compute_point` applies the inverse of every intermediate
    /// allocation transform (scale and rotation), so delegating to
    /// the innermost card yields the exact hit region for free.
    fn contains(&self, x: f64, y: f64) -> bool {
        let widget = self.obj();
        let Some(card) = widget.first_child().and_then(|child| child.first_child()) else {
            return false;
        };
        widget
            .compute_point(&card, &gtk4::graphene::Point::new(x as f32, y as f32))
            .map(|point| card.contains(f64::from(point.x()), f64::from(point.y())))
            .unwrap_or_default()
    }
}

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
    fn natural_child_size(&self, widget: &gtk4::Widget) -> Size<i32> {
        let Some(child) = widget.first_child() else {
            return Size::new(0, 0);
        };
        child.measure_size().1
    }
}

impl LayoutManagerImpl for ScalingLayoutImpl {
    fn measure(&self, widget: &gtk4::Widget, orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        let scale = self.scale.get();
        if let Some(child) = widget.first_child() {
            let extent = (child.measure_axis(orientation, for_size).to_f64() * scale).to_i32_ceil();
            (extent.x, extent.y, -1, -1)
        } else {
            (0, 0, -1, -1)
        }
    }

    fn allocate(&self, widget: &gtk4::Widget, width: i32, height: i32, baseline: i32) {
        let Some(child) = widget.first_child() else {
            return;
        };
        let allocated = Size::new(width, height);
        let natural = self.natural_child_size(widget);
        let child_size = Size::new(if natural.x > 0 { natural.x } else { allocated.x }, if natural.y > 0 { natural.y } else { allocated.y });
        // Center the unscaled child in the allocation, then scale around the center.
        let transform = Transform::from(CenteredTransform::new(
            allocated.to_f32().half(),
            child_size.to_f32(),
            RotationRad::ZERO,
            ScaleFactor::new(self.scale.get()),
        ));
        child.allocate(child_size.x, child_size.y, baseline, Some(transform));
    }
}

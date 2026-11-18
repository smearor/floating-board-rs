//! `FloatingBoardLayoutImpl` - passthrough layout manager that hooks
//! the board's allocation pass.

use crate::widget::board::FloatingBoard;
use crate::widget::floating_board_layout::FloatingBoardLayout;
use glib::Properties;
use glib::object_subclass;
use glib::prelude::Cast;
use glib::prelude::ObjectExt;
use glib::subclass::object::DerivedObjectProperties;
use gtk4::Orientation;
use gtk4::SizeRequestMode;
use gtk4::Widget;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::LayoutManagerImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::OrientableImpl;
use std::cell::Cell;

/// Implementation of [`FloatingBoardLayout`].
///
/// `Orientable` is implemented because `GtkBox` delegates its
/// `orientation` property to the layout manager - get/set without the
/// interface raises `GTK_IS_ORIENTABLE` criticals. The property is
/// stored but unused: the board gives every child the full area
/// regardless of orientation.
#[derive(Properties)]
#[properties(wrapper_type = FloatingBoardLayout)]
pub struct FloatingBoardLayoutImpl {
    /// Box-compatible orientation property - accepted, not interpreted.
    #[property(get, set, builder(Orientation::Vertical))]
    orientation: Cell<Orientation>,
}

impl Default for FloatingBoardLayoutImpl {
    fn default() -> Self {
        Self {
            orientation: Cell::new(Orientation::Vertical),
        }
    }
}

#[object_subclass]
impl ObjectSubclass for FloatingBoardLayoutImpl {
    const NAME: &'static str = "FloatingBoardLayout";
    type Type = FloatingBoardLayout;
    type ParentType = gtk4::LayoutManager;
    type Interfaces = (gtk4::Orientable,);
}

#[glib::derived_properties]
impl ObjectImpl for FloatingBoardLayoutImpl {}

impl OrientableImpl for FloatingBoardLayoutImpl {}

impl LayoutManagerImpl for FloatingBoardLayoutImpl {
    /// The children merge like an overlay, not a box: each is measured
    /// in the same `for_size` and the board takes the largest request.
    /// The board manages exactly one overlay child, so this reduces to
    /// a passthrough.
    fn request_mode(&self, widget: &Widget) -> SizeRequestMode {
        let mut mode = SizeRequestMode::ConstantSize;
        let mut child = widget.first_child();
        while let Some(current) = child {
            match current.request_mode() {
                SizeRequestMode::HeightForWidth => return SizeRequestMode::HeightForWidth,
                SizeRequestMode::WidthForHeight => mode = SizeRequestMode::WidthForHeight,
                _ => {}
            }
            child = current.next_sibling();
        }
        mode
    }

    fn measure(&self, widget: &Widget, orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        let (mut minimum, mut natural, mut minimum_baseline, mut natural_baseline) = (0, 0, -1, -1);
        let mut child = widget.first_child();
        while let Some(current) = child {
            let (child_minimum, child_natural, child_minimum_baseline, child_natural_baseline) = current.measure(orientation, for_size);
            minimum = minimum.max(child_minimum);
            natural = natural.max(child_natural);
            minimum_baseline = minimum_baseline.max(child_minimum_baseline);
            natural_baseline = natural_baseline.max(child_natural_baseline);
            child = current.next_sibling();
        }
        (minimum, natural, minimum_baseline, natural_baseline)
    }

    /// Gives every child the full board area, then runs the board's
    /// per-allocation reconciliation: pending anchors resolve, card
    /// transforms sync to the workspace view and the native input
    /// region is restored (see [`FloatingBoard::on_board_allocated`]).
    /// This is the only allocation hook GTK4 calls for the board - the
    /// `size_allocate` vfunc is bypassed because `GtkBox` installs a
    /// layout manager.
    fn allocate(&self, widget: &Widget, width: i32, height: i32, baseline: i32) {
        let allocation = gtk4::Allocation::new(0, 0, width, height);
        let mut child = widget.first_child();
        while let Some(current) = child {
            current.size_allocate(&allocation, baseline);
            child = current.next_sibling();
        }
        if let Some(board) = widget.downcast_ref::<FloatingBoard>() {
            board.on_board_allocated();
        }
    }
}

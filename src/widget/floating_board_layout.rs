//! `FloatingBoardLayout` - the board's layout manager, which also
//! drives the per-allocation board reconciliation.

use crate::widget::imp::floating_board_layout::FloatingBoardLayoutImpl;
use gtk4::glib;

glib::wrapper! {
    /// Layout manager of [`FloatingBoard`](crate::widget::FloatingBoard):
    /// gives every child the full board area (the board is a surface,
    /// not a stacking container - it manages exactly one overlay child)
    /// and runs the board's position reconciliation each allocation
    /// pass, where a plain `size_allocate` vfunc would never be called
    /// (GTK4 delegates straight to the layout manager for widgets that
    /// have one, like `GtkBox`).
    pub struct FloatingBoardLayout(ObjectSubclass<FloatingBoardLayoutImpl>)
        @extends gtk4::LayoutManager,
        @implements gtk4::Orientable;
}

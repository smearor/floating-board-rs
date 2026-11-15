//! `CardLayerImpl` - the `gtk4::Fixed` card layer that only lets its
//! children pick events.

use crate::widget::CardLayer;
use crate::widget::FloatingBoard;
use glib::WeakRef;
use glib::object_subclass;
use gtk4::subclass::prelude::FixedImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::WidgetImpl;
use gtk4::subclass::prelude::WidgetImplExt;
use std::cell::RefCell;

/// Implementation of [`CardLayer`].
#[derive(Default)]
pub struct CardLayerImpl {
    /// Owning board - reconciles card positions while children
    /// re-allocate (see `size_allocate`).
    pub board: RefCell<Option<WeakRef<FloatingBoard>>>,
}

#[object_subclass]
impl ObjectSubclass for CardLayerImpl {
    const NAME: &'static str = "FloatingBoardCardLayer";
    type Type = CardLayer;
    type ParentType = gtk4::Fixed;
}

impl ObjectImpl for CardLayerImpl {}

impl WidgetImpl for CardLayerImpl {
    /// The layer itself is never an event target - only the cards
    /// placed on it are. GTK pick checks children *before* consulting
    /// `contains` for the parent itself, so reporting `false` here
    /// passes clicks on empty areas through to the content below
    /// while the cards stay interactive.
    fn contains(&self, _x: f64, _y: f64) -> bool {
        false
    }

    /// A `gtk4::Fixed` re-allocates whenever a child's measured size
    /// changes - the board's own allocation is unaffected, so its
    /// position reconciliation would miss the pass. Re-applying the
    /// stored `put` positions here keeps grown or rotated cards centered
    /// on their model position in the same layout pass that allocates
    /// their new size (otherwise the card renders off its footprint and
    /// off the input region until the board reallocates).
    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        if let Some(board) = self.board.borrow().as_ref().and_then(WeakRef::upgrade) {
            board.reconcile_card_positions();
        }
        self.parent_size_allocate(width, height, baseline);
    }
}

impl FixedImpl for CardLayerImpl {}

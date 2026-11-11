use crate::domain::CardId;
use crate::domain::SequenceNumber;
use crate::widget::board::FloatingBoard;
use crate::widget::board_signal::BOARD_SIGNALS;
use crate::widget::card_layer::CardLayer;
use crate::widget::css::CLASS_BOARD;
use crate::widget::css::CLASS_CARD_LAYER;
use crate::widget::imp::card_entry::CardEntry;
use glib::subclass::Signal;
use gtk4::Orientation;
use gtk4::glib;
use gtk4::prelude::BoxExt;
use gtk4::prelude::NativeExt;
use gtk4::prelude::OrientableExt;
use gtk4::prelude::SurfaceExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::BoxImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectImplExt;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::prelude::WidgetImpl;
use gtk4::subclass::prelude::WidgetImplExt;
use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// Widget implementation for [`FloatingBoard`].
pub struct FloatingBoardImpl {
    /// Internal overlay stacking the card layer over the content.
    pub overlay: OnceCell<gtk4::Overlay>,
    /// Transparent card layer holding the card transform stacks.
    /// Empty areas have no pickable, so gestures fall through to the content.
    pub card_layer: OnceCell<CardLayer>,
    /// All cards keyed by id (sorted, for deterministic snapshots).
    pub(crate) cards: RefCell<BTreeMap<CardId, CardEntry>>,
    /// Counter for auto-generated `card-N` ids and insertion order.
    pub sequence: Cell<SequenceNumber>,
    /// When `true`, an external consumer manages the surface input region
    /// and `size_allocate` stops resetting it.
    pub input_region_managed: Cell<bool>,
}

impl Default for FloatingBoardImpl {
    fn default() -> Self {
        Self {
            overlay: OnceCell::new(),
            card_layer: OnceCell::new(),
            cards: RefCell::new(BTreeMap::new()),
            sequence: Cell::new(SequenceNumber::default()),
            input_region_managed: Cell::new(false),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for FloatingBoardImpl {
    const NAME: &'static str = "FloatingBoard";
    type Type = FloatingBoard;
    type ParentType = gtk4::Box;
}

impl ObjectImpl for FloatingBoardImpl {
    fn constructed(&self) {
        self.parent_constructed();
        let board = self.obj();
        board.set_orientation(Orientation::Vertical);
        board.add_css_class(CLASS_BOARD);

        let overlay = gtk4::Overlay::new();
        overlay.set_hexpand(true);
        overlay.set_vexpand(true);

        let card_layer = CardLayer::new();
        card_layer.add_css_class(CLASS_CARD_LAYER);
        overlay.add_overlay(&card_layer);

        board.append(&overlay);
        let _ = self.card_layer.set(card_layer);
        let _ = self.overlay.set(overlay);
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| BOARD_SIGNALS.iter().map(|signal| signal.signal()).collect());
        &SIGNALS
    }
}

impl WidgetImpl for FloatingBoardImpl {
    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.parent_size_allocate(width, height, baseline);
        // smearor-wrot-rotation is designed for layer-shell surfaces and
        // narrows the surface input region to each card's footprint on every
        // allocation. Inside the board all card allocations run through this
        // vfunc, so restoring the full input region here wins per layout pass.
        // Skipped while a consumer manages the region itself
        // (`set_input_region_managed`) - e.g. touch-passthrough overlays that
        // restrict input to the cards.
        if !self.input_region_managed.get()
            && let Some(surface) = self.obj().native().and_then(|native| native.surface())
        {
            surface.set_input_region(None);
        }
        self.obj().on_board_allocated();
    }
}

impl BoxImpl for FloatingBoardImpl {}

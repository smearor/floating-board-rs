use crate::domain::CardId;
use crate::domain::GenerationCounter;
use crate::domain::SeatAnchor;
use crate::domain::SequenceNumber;
use crate::widget::board::FloatingBoard;
use crate::widget::board_signal::BOARD_SIGNALS;
use crate::widget::card_layer::CardLayer;
use crate::widget::css::CLASS_BOARD;
use crate::widget::css::CLASS_CARD_LAYER;
use crate::widget::floating_board_layout::FloatingBoardLayout;
use crate::widget::imp::card_entry::CardEntry;
use glib::subclass::Signal;
use gtk4::Orientation;
use gtk4::glib;
use gtk4::prelude::BoxExt;
use gtk4::prelude::OrientableExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::BoxImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectImplExt;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::prelude::WidgetClassExt;
use gtk4::subclass::prelude::WidgetImpl;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::RotationRad;
use smearor_commons_geometry::ScaleFactor;
use smearor_commons_geometry::Vec2;
use smearor_commons_layout::View;
use smearor_commons_layout::WorkspaceSize;
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
    /// Extent mode of the virtual workspace the cards live on.
    pub workspace_size: Cell<WorkspaceSize<Pixels>>,
    /// The configured camera over the workspace; `None` resolves to the
    /// default view covering the workspace extent (identity for the
    /// `FollowWidget` default). An explicit `Some` is pinned and
    /// survives resizes; `reset_view` clears back to `None`.
    pub view: RefCell<Option<View<Pixels>>>,
    /// Generation of the running `goto_card` camera flight; a new
    /// flight or any programmatic/gesture view change bumps it so
    /// stale ticks stop themselves.
    pub view_flight: Cell<GenerationCounter>,
    /// Empty-space gestures drive the view camera (pan, pinch zoom,
    /// two-finger rotate) while set - default `false`, see
    /// `FloatingBoard::set_view_gestures`.
    pub view_gestures: Cell<bool>,
    /// The corner view-control button box is visible while set -
    /// default `false`, see `FloatingBoard::set_view_buttons`.
    pub view_buttons: Cell<bool>,
    /// Button box overlaid on the bottom-right corner (zoom in, zoom
    /// out, reset). Created in `constructed`; visibility follows
    /// `view_buttons`.
    pub view_controls: OnceCell<gtk4::Box>,
    /// Last cumulative offset reported by the view pan drag (layer
    /// pixels since drag begin) - the workspace delta is computed
    /// between consecutive offsets so the workspace point stays under
    /// the finger while the view pans.
    pub view_pan_offset: Cell<Vec2<Pixels>>,
    /// Last cumulative scale reported by the view zoom gesture.
    pub view_zoom_scale: Cell<ScaleFactor>,
    /// Last cumulative angle delta reported by the view rotate gesture.
    pub view_rotate_delta: Cell<RotationRad>,
    /// Where pass-to-seat operations anchor seat edges - default
    /// `SeatAnchor::View` (the screen edge where users sit).
    pub seat_anchor: Cell<SeatAnchor>,
}

impl Default for FloatingBoardImpl {
    fn default() -> Self {
        Self {
            overlay: OnceCell::new(),
            card_layer: OnceCell::new(),
            cards: RefCell::new(BTreeMap::new()),
            sequence: Cell::new(SequenceNumber::default()),
            input_region_managed: Cell::new(false),
            workspace_size: Cell::new(WorkspaceSize::default()),
            view: RefCell::new(None),
            view_flight: Cell::new(GenerationCounter::default()),
            view_gestures: Cell::new(false),
            view_buttons: Cell::new(false),
            view_controls: OnceCell::new(),
            view_pan_offset: Cell::new(Vec2::ZERO),
            view_zoom_scale: Cell::new(ScaleFactor::ONE),
            view_rotate_delta: Cell::new(RotationRad::ZERO),
            seat_anchor: Cell::new(SeatAnchor::default()),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for FloatingBoardImpl {
    const NAME: &'static str = "FloatingBoard";
    type Type = FloatingBoard;
    type ParentType = gtk4::Box;

    fn class_init(klass: &mut Self::Class) {
        // `GtkBox` installs a `GtkBoxLayout`, and for widgets with a
        // layout manager GTK4 calls `gtk_layout_manager_allocate`
        // directly - a `size_allocate` vfunc would never run. The
        // board therefore carries its own layout manager, whose
        // `allocate` drives the per-pass card reconciliation.
        klass.set_layout_manager_type::<FloatingBoardLayout>();
    }
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
        // The layer must fill the board: a fixed-style container
        // otherwise measures at its children bounding box, shrinking
        // and shifting with card positions (picking and any geometry
        // derived from its allocation would follow the card cluster).
        card_layer.set_hexpand(true);
        card_layer.set_vexpand(true);
        overlay.add_overlay(&card_layer);

        board.append(&overlay);
        let _ = self.card_layer.set(card_layer);
        let _ = self.overlay.set(overlay);

        board.wire_view_gestures();
        board.wire_view_controls();
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| BOARD_SIGNALS.iter().map(|signal| signal.signal()).collect());
        &SIGNALS
    }
}

impl WidgetImpl for FloatingBoardImpl {}

impl BoxImpl for FloatingBoardImpl {}

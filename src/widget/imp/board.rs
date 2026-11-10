use crate::domain::CardAnchor;
use crate::domain::CardId;
use crate::domain::CardPosition;
use crate::domain::ScaleFactor;
use crate::widget::board::FloatingBoard;
use crate::widget::card::Card;
use crate::widget::card_config::CardConfig;
use crate::widget::card_layer::CardLayer;
use crate::widget::css::CLASS_BOARD;
use crate::widget::css::CLASS_CARD_LAYER;
use crate::widget::scaling::ScalingWidget;
use glib::prelude::StaticType;
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
use smearor_wrot_rotation::RotationWidget;
use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// Snapshot of the card geometry and quad points taken when a quad
/// transform (re-)activates; refreshed whenever the point count changes so
/// adding or lifting fingers never causes a jump.
pub(crate) struct QuadBaseline {
    /// Card center when the baseline was taken (board coordinates).
    pub position: CardPosition,
    /// Scale factor at baseline.
    pub scale: f64,
    /// Rotation in degrees at baseline.
    pub rotation_deg: f64,
    /// Centroid of `points` (board coordinates).
    pub centroid: (f64, f64),
    /// Board-space point per active quad slot at baseline.
    pub points: Vec<(f64, f64)>,
    /// Whether the transform already changed the position.
    pub moved: bool,
    /// Whether the transform already changed the rotation.
    pub rotated: bool,
    /// Whether the transform already changed the scale.
    pub scaled: bool,
    /// The angle applied by the most recent transform update, used to
    /// detect degenerate finger crossings (~180° flips between frames).
    pub applied_angle_deg: f64,
}

/// One finger tracked by the quad transform: the touch sequence, its latest
/// board-space point and when the last event for it arrived (the timestamp
/// detects touches whose `TouchEnd` was routed to a claiming gesture and
/// never reached the tracker).
#[derive(Debug, Clone)]
pub(crate) struct TrackedTouch {
    /// The touch sequence identifying this finger.
    pub sequence: gtk4::gdk::EventSequence,
    /// Latest finger position in board coordinates.
    pub point: (f64, f64),
    /// Timestamp of the most recent event for this sequence.
    pub last_event: std::time::Instant,
}

/// Board-owned state of one card: the transform stack widgets plus the
/// geometry cells. `Card` objects carry only a weak backlink - this entry is
/// the single owner of position/scale state (rotation lives inside the
/// `RotationWidget`'s layout and is read on demand).
pub(crate) struct CardEntry {
    /// The card view widget.
    pub card: Card,
    /// Outer transform: scale wrapper placed on the `gtk4::Fixed` layer.
    pub scaling: ScalingWidget,
    /// Inner transform: rotation container holding the card.
    pub rotation_widget: RotationWidget,
    /// Immutable configuration passed at `add_card`.
    pub config: CardConfig,
    /// Current center position in board coordinates.
    pub position: Cell<CardPosition>,
    /// Current scale factor.
    pub scale: Cell<ScaleFactor>,
    /// Anchor awaiting resolution at the next board allocation.
    pub pending_anchor: RefCell<Option<CardAnchor>>,
    /// Card center at the start of the active drag gesture.
    pub drag_start: Cell<CardPosition>,
    /// Pointer position in board coordinates at drag begin.
    pub drag_grab: Cell<Option<(f64, f64)>>,
    /// Live touch points for the quad transform, one entry per finger
    /// currently on the card. The tracker's `EventControllerLegacy` keeps
    /// this in sync - a plain event observer that sees every sequence
    /// without claiming it, so interactive content keeps working (unlike
    /// `GestureSingle`, which locks onto the first sequence it sees and
    /// cannot distribute touches across siblings).
    pub quad_touches: RefCell<Vec<TrackedTouch>>,
    /// Baseline for the active quad transform.
    pub quad_baseline: RefCell<Option<QuadBaseline>>,
    /// Insertion sequence number for stable z-ordering.
    pub sequence: u64,
}

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
    pub sequence: Cell<u64>,
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
            sequence: Cell::new(0),
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
        static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| {
            vec![
                Signal::builder("card-added").param_types([Card::static_type()]).build(),
                Signal::builder("card-removed").param_types([Card::static_type()]).build(),
                Signal::builder("geometry-changed").build(),
            ]
        });
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

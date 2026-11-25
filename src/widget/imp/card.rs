use crate::domain::CardId;
use crate::domain::CardShape;
use crate::domain::CircleContent;
use crate::widget::FloatingBoard;
use crate::widget::card::Card;
use crate::widget::card_layout::CardShapeLayout;
use crate::widget::card_signal::CARD_SIGNALS;
use crate::widget::css::CLASS_CARD;
use crate::widget::css::CLASS_CONTENT;
use glib::WeakRef;
use glib::subclass::Signal;
use gtk4::Button;
use gtk4::DrawingArea;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Overflow;
use gtk4::Overlay;
use gtk4::Widget;
use gtk4::glib;
use gtk4::graphene::Point;
use gtk4::gsk::Transform;
use gtk4::prelude::BoxExt;
use gtk4::prelude::Cast;
use gtk4::prelude::OrientableExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::BoxImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectImplExt;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::prelude::WidgetClassExt;
use gtk4::subclass::prelude::WidgetImpl;
use gtk4::subclass::prelude::WidgetImplExt;
use smearor_commons_geometry::Circle;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::sync::LazyLock;

/// Widget implementation for [`Card`].
///
/// The card is a view: it owns no geometry state. Position/rotation/scale live
/// exclusively on the `FloatingBoard`; the backlink here only delegates calls.
pub struct CardImpl {
    /// Semantic identifier of this card.
    pub id: RefCell<CardId>,
    /// Weak backlink to the owning board.
    pub board: RefCell<WeakRef<FloatingBoard>>,
    /// The installed palette's generated CSS class + provider, so
    /// `Card::set_palette` can uninstall a replaced palette.
    pub palette: RefCell<Option<(String, gtk4::CssProvider)>>,
    /// Header strip (drag affordance) - built by `chrome::mount_chrome`
    /// during `Card::build`; `gtk4::Box` for the builtin style,
    /// `adw::HeaderBar` for adwaita.
    pub header: OnceCell<gtk4::Widget>,
    /// Whether the header has chrome children (and may be shown);
    /// `set_folded` restores to this instead of unconditionally
    /// revealing an empty strip.
    pub header_shown: Cell<bool>,
    /// Overlay stacking the resize/rotate grips over the content.
    pub content_overlay: OnceCell<Overlay>,
    /// Container for the consumer's content widget.
    pub content_holder: OnceCell<gtk4::Box>,
    /// Resolved title - source of truth for `Card::title` regardless
    /// of which chrome renderer displays it.
    pub title_text: RefCell<String>,
    /// Title label inside the header (builtin style only).
    pub title_label: OnceCell<Label>,
    /// Grip icon used in `DragHandle::Handle` mode.
    pub handle_label: OnceCell<Label>,
    /// Fold toggle button (visible when `CardConfig::foldable`).
    pub fold_button: OnceCell<Button>,
    /// Close button (visible when `CardConfig::closable`).
    pub close_button: OnceCell<Button>,
    /// Bottom-right resize grip (`CardConfig::resizable`) - hidden by
    /// default, shown from `Card::build` and while unfolded.
    pub resize_grip: OnceCell<DrawingArea>,
    /// Whether the card offers the resize grip - kept so `set_folded`
    /// can gate the grip's visibility on the fold state.
    pub resizable: Cell<bool>,
    /// Bottom-left rotate grip (`CardConfig::rotate_grip`) - hidden by
    /// default, shown from `Card::build` and while unfolded.
    pub rotate_grip: OnceCell<DrawingArea>,
    /// Whether the card offers the rotate grip - kept so `set_folded`
    /// can gate the grip's visibility on the fold state.
    pub rotate_grip_enabled: Cell<bool>,
    /// Whether the card is currently folded to its header.
    pub folded: Cell<bool>,
    /// Container for `CardConfig::folded_content` - visible only while
    /// folded with a custom folded widget installed.
    pub folded_holder: OnceCell<gtk4::Box>,
    /// Upper bound on the reported width (`CardConfig::max_width`).
    pub max_width: Cell<Option<Pixels>>,
    /// Upper bound on the reported height (`CardConfig::max_height`).
    pub max_height: Cell<Option<Pixels>>,
    /// Fold-morph measure override (`CardFoldAnimation`): while a
    /// morph runs, `CardShapeLayout` reports this pinned size instead
    /// of measuring children - a `size_request` cannot shrink a card
    /// below its content's own minimum, so the footprint tween needs
    /// the layout-level override. `apply_folded_presentation` clears it.
    pub morph_measure: Cell<Option<Vec2<Pixels, i32>>>,
    /// The card's outline (`CardConfig::shape`) - drives the circle
    /// branches in `measure`/`size_allocate`/`contains`.
    pub shape: Cell<CardShape>,
    /// Content allocation mode for `CardShape::Circle`
    /// (`CardConfig::circle_content`).
    pub circle_content: Cell<CircleContent>,
}

impl Default for CardImpl {
    fn default() -> Self {
        Self {
            id: RefCell::new(CardId::new("")),
            board: RefCell::new(WeakRef::new()),
            palette: RefCell::new(None),
            header: OnceCell::new(),
            header_shown: Cell::new(false),
            content_overlay: OnceCell::new(),
            content_holder: OnceCell::new(),
            title_text: RefCell::new(String::new()),
            title_label: OnceCell::new(),
            handle_label: OnceCell::new(),
            fold_button: OnceCell::new(),
            close_button: OnceCell::new(),
            resize_grip: OnceCell::new(),
            resizable: Cell::new(false),
            rotate_grip: OnceCell::new(),
            rotate_grip_enabled: Cell::new(false),
            folded: Cell::new(false),
            folded_holder: OnceCell::new(),
            morph_measure: Cell::new(None),
            max_width: Cell::new(None),
            max_height: Cell::new(None),
            shape: Cell::new(CardShape::default()),
            circle_content: Cell::new(CircleContent::default()),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for CardImpl {
    const NAME: &'static str = "FloatingBoardCard";
    type Type = Card;
    type ParentType = gtk4::Box;

    fn class_init(klass: &mut Self::Class) {
        // Shape-aware layout: GTK4 consults the layout manager for
        // `measure`/`allocate` before the widget vfuncs, and `GtkBox`
        // always carries one - the circle sizing only works from here.
        klass.set_layout_manager_type::<CardShapeLayout>();
    }
}

impl ObjectImpl for CardImpl {
    fn constructed(&self) {
        self.parent_constructed();
        let card = self.obj();
        card.set_orientation(Orientation::Vertical);
        // No `set_spacing(0)`: it delegates to `gtk_box_layout_*` and
        // hard-asserts on our custom `CardShapeLayout`; the delegate
        // inside defaults to spacing 0 anyway.
        // Clips content that exceeds a `max_width`/`max_height` cap and keeps
        // it inside the rounded corners (GTK CSS has no `overflow` property).
        card.set_overflow(Overflow::Hidden);
        card.add_css_class(CLASS_CARD);

        // The header strip is NOT built here: `chrome::mount_chrome`
        // composes it during `Card::build` from the config's chrome
        // slot list (and prepends it before the content overlay).
        let content_holder = gtk4::Box::new(Orientation::Vertical, 0);
        content_holder.add_css_class(CLASS_CONTENT);
        content_holder.set_vexpand(true);

        // The overlay stacks the resize/rotate grips over the
        // content's corners without claiming layout space.
        let content_overlay = Overlay::new();
        content_overlay.set_child(Some(&content_holder));

        let folded_holder = gtk4::Box::new(Orientation::Horizontal, 0);
        folded_holder.add_css_class(crate::widget::css::CLASS_FOLDED_CONTENT);
        folded_holder.set_visible(false);
        folded_holder.set_halign(gtk4::Align::Center);
        folded_holder.set_valign(gtk4::Align::Center);

        // The stage stacks the folded pane over the content pane:
        // while folded - or mid-crossfade in a `CardFoldAnimation`
        // transition - both panes share the same area. The folded pane
        // is an overlay child, so it never claims layout space; the
        // folded state's size is pinned explicitly in `set_folded`.
        let stage = Overlay::new();
        stage.set_child(Some(&content_overlay));
        stage.add_overlay(&folded_holder);

        card.append(&stage);

        let _ = self.content_overlay.set(content_overlay);
        let _ = self.content_holder.set(content_holder);
        let _ = self.folded_holder.set(folded_holder);
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| CARD_SIGNALS.iter().map(|signal| signal.signal()).collect());
        &SIGNALS
    }
}

/// Width of the ring band around a circle card's inner circle (px).
/// The band is card chrome: it carries the card's background and stays
/// a drag affordance; the content never reaches into it.
const CIRCLE_RING_BAND: f64 = 8.0;

impl WidgetImpl for CardImpl {
    // `Circle`: only the disk inside the bounding square is pickable -
    // taps in the corners pass through to whatever is underneath
    // (through the board layers' `contains` chain, same as the rotated
    // corners of a rectangle card).
    fn contains(&self, x: f64, y: f64) -> bool {
        if self.shape.get() == CardShape::Circle {
            let (width, height) = (f64::from(self.obj().width()), f64::from(self.obj().height()));
            return Circle::<()>::new(Vec2::new(width / 2.0, height / 2.0), width.min(height) / 2.0).contains(&Vec2::new(x, y));
        }
        self.parent_contains(x, y)
    }
}

impl CardImpl {
    /// `Circle` measure: the card reports its bounding square - a
    /// diameter derived from the visible content child's natural size
    /// via `CircleContent::diameter_for` (the `folded_holder` while it
    /// carries a collapsed widget, else the `content_overlay`). Called
    /// from `CardShapeLayout`.
    pub(crate) fn circle_measure(&self) -> (i32, i32, i32, i32) {
        let content = self
            .folded_holder
            .get()
            .filter(|holder| holder.is_visible() && holder.first_child().is_some())
            .map(|holder| holder.clone().upcast::<Widget>())
            .or_else(|| self.content_overlay.get().map(|overlay| overlay.clone().upcast()));
        let Some(content) = content else {
            return (0, 0, -1, -1);
        };
        let (_, natural_width, _, _) = content.measure(Orientation::Horizontal, -1);
        let (_, natural_height, _, _) = content.measure(Orientation::Vertical, -1);
        // A disc does not flex: minimum == natural keeps the bounding
        // square stable (the parent layer allocates at the minimum
        // width - a smaller value would render a shrunken circle whose
        // satellites still orbit the natural diameter).
        let diameter = self
            .circle_content
            .get()
            .diameter_for(f64::from(natural_width.max(natural_height)), CIRCLE_RING_BAND);
        (diameter.ceil() as i32, diameter.ceil() as i32, -1, -1)
    }

    /// `Circle` allocate: every visible child gets the centered content
    /// square (`CircleContent::content_square_for` of the allocated
    /// diameter); the margin around it renders as the ring. Called from
    /// `CardShapeLayout`.
    pub(crate) fn circle_allocate(&self, width: i32, height: i32, baseline: i32) {
        let diameter = f64::from(width.min(height));
        let side = self.circle_content.get().content_square_for(diameter, CIRCLE_RING_BAND);
        let transform = Transform::new().translate(&Point::new(((f64::from(width) - side) / 2.0) as f32, ((f64::from(height) - side) / 2.0) as f32));
        let side = side.ceil() as i32;
        let mut child = self.obj().first_child();
        while let Some(widget) = child {
            if widget.is_visible() {
                widget.allocate(side, side, baseline, Some(transform.clone()));
            }
            child = widget.next_sibling();
        }
    }
}

impl BoxImpl for CardImpl {}

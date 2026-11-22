use crate::domain::CardId;
use crate::widget::FloatingBoard;
use crate::widget::card::Card;
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
use gtk4::glib;
use gtk4::prelude::BoxExt;
use gtk4::prelude::OrientableExt;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::BoxImpl;
use gtk4::subclass::prelude::ObjectImpl;
use gtk4::subclass::prelude::ObjectImplExt;
use gtk4::subclass::prelude::ObjectSubclass;
use gtk4::subclass::prelude::ObjectSubclassExt;
use gtk4::subclass::prelude::WidgetImpl;
use gtk4::subclass::prelude::WidgetImplExt;
use smearor_commons_geometry::Pixels;
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
}

impl Default for CardImpl {
    fn default() -> Self {
        Self {
            id: RefCell::new(CardId::new("")),
            board: RefCell::new(WeakRef::new()),
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
            max_width: Cell::new(None),
            max_height: Cell::new(None),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for CardImpl {
    const NAME: &'static str = "FloatingBoardCard";
    type Type = Card;
    type ParentType = gtk4::Box;
}

impl ObjectImpl for CardImpl {
    fn constructed(&self) {
        self.parent_constructed();
        let card = self.obj();
        card.set_orientation(Orientation::Vertical);
        card.set_spacing(0);
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

        card.append(&content_overlay);
        card.append(&folded_holder);

        let _ = self.content_overlay.set(content_overlay);
        let _ = self.content_holder.set(content_holder);
        let _ = self.folded_holder.set(folded_holder);
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| CARD_SIGNALS.iter().map(|signal| signal.signal()).collect());
        &SIGNALS
    }
}

impl WidgetImpl for CardImpl {
    // Caps the reported size at `CardConfig::max_width`/`max_height` so the
    // card stops growing when its content grows; the parent layer allocates
    // at most the capped size and the default CSS clips the overflow.
    fn measure(&self, orientation: Orientation, for_size: i32) -> (i32, i32, i32, i32) {
        let (minimum, natural, minimum_baseline, natural_baseline) = self.parent_measure(orientation, for_size);
        let limit = match orientation {
            Orientation::Horizontal => self.max_width.get(),
            Orientation::Vertical => self.max_height.get(),
            _ => None,
        };
        match limit {
            // Fractional caps floor so the reported size never exceeds the bound.
            Some(cap) => {
                let cap = cap.as_f64() as i32;
                (minimum.min(cap), natural.min(cap), -1, -1)
            }
            None => (minimum, natural, minimum_baseline, natural_baseline),
        }
    }
}

impl BoxImpl for CardImpl {}

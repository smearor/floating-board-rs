use crate::domain::CardId;
use crate::widget::FloatingBoard;
use crate::widget::card::Card;
use crate::widget::card_signal::CARD_SIGNALS;
use crate::widget::css::CLASS_CARD;
use crate::widget::css::CLASS_CONTENT;
use crate::widget::css::CLASS_HANDLE;
use crate::widget::css::CLASS_HEADER;
use glib::WeakRef;
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
use gtk4::subclass::prelude::WidgetImpl;
use gtk4::subclass::prelude::WidgetImplExt;
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
    /// Header strip (drag affordance), built in `constructed`.
    pub header: OnceCell<gtk4::Box>,
    /// Container for the consumer's content widget.
    pub content_holder: OnceCell<gtk4::Box>,
    /// Title label inside the header.
    pub title_label: OnceCell<gtk4::Label>,
    /// Grip icon used in `DragHandle::Handle` mode.
    pub handle_label: OnceCell<gtk4::Label>,
    /// Fold toggle button (visible when `CardConfig::foldable`).
    pub fold_button: OnceCell<gtk4::Button>,
    /// Close button (visible when `CardConfig::closable`).
    pub close_button: OnceCell<gtk4::Button>,
    /// Whether the card is currently folded to its header.
    pub folded: Cell<bool>,
    /// Upper bound on the reported width (`CardConfig::max_width`).
    pub max_width: Cell<Option<i32>>,
    /// Upper bound on the reported height (`CardConfig::max_height`).
    pub max_height: Cell<Option<i32>>,
}

impl Default for CardImpl {
    fn default() -> Self {
        Self {
            id: RefCell::new(CardId::new("")),
            board: RefCell::new(WeakRef::new()),
            header: OnceCell::new(),
            content_holder: OnceCell::new(),
            title_label: OnceCell::new(),
            handle_label: OnceCell::new(),
            fold_button: OnceCell::new(),
            close_button: OnceCell::new(),
            folded: Cell::new(false),
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
        card.set_overflow(gtk4::Overflow::Hidden);
        card.add_css_class(CLASS_CARD);

        let header = gtk4::Box::new(Orientation::Horizontal, 6);
        header.add_css_class(CLASS_HEADER);
        header.set_hexpand(true);

        let handle_label = gtk4::Label::new(Some("\u{2261}"));
        handle_label.add_css_class(CLASS_HANDLE);
        handle_label.set_visible(false);
        header.append(&handle_label);

        let title_label = gtk4::Label::new(None);
        title_label.set_hexpand(true);
        title_label.set_halign(gtk4::Align::Start);
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        header.append(&title_label);

        let fold_button = gtk4::Button::from_icon_name("go-down-symbolic");
        fold_button.add_css_class("flat");
        fold_button.set_visible(false);
        header.append(&fold_button);

        let close_button = gtk4::Button::from_icon_name("window-close-symbolic");
        close_button.add_css_class("flat");
        close_button.set_visible(false);
        header.append(&close_button);

        let content_holder = gtk4::Box::new(Orientation::Vertical, 0);
        content_holder.add_css_class(CLASS_CONTENT);
        content_holder.set_vexpand(true);

        card.append(&header);
        card.append(&content_holder);

        let _ = self.header.set(header);
        let _ = self.content_holder.set(content_holder);
        let _ = self.title_label.set(title_label);
        let _ = self.handle_label.set(handle_label);
        let _ = self.fold_button.set(fold_button);
        let _ = self.close_button.set(close_button);
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
            Some(cap) => (minimum.min(cap), natural.min(cap), -1, -1),
            None => (minimum, natural, minimum_baseline, natural_baseline),
        }
    }
}

impl BoxImpl for CardImpl {}

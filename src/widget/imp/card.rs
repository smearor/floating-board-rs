use crate::domain::CardId;
use crate::widget::FloatingBoard;
use crate::widget::card::Card;
use crate::widget::card_signal::CARD_SIGNALS;
use crate::widget::css::CLASS_CARD;
use crate::widget::css::CLASS_CONTENT;
use crate::widget::css::CLASS_HANDLE;
use crate::widget::css::CLASS_HEADER;
use crate::widget::css::CLASS_RESIZE_GRIP;
use crate::widget::css::CLASS_ROTATE_GRIP;
use glib::WeakRef;
use glib::subclass::Signal;
use gtk4::Align;
use gtk4::Button;
use gtk4::DrawingArea;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Overflow;
use gtk4::Overlay;
use gtk4::cairo::LineCap;
use gtk4::glib;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::BoxExt;
use gtk4::prelude::DrawingAreaExt;
use gtk4::prelude::DrawingAreaExtManual;
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
use std::f64::consts::FRAC_PI_2;
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
            content_holder: OnceCell::new(),
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

        let header = gtk4::Box::new(Orientation::Horizontal, 6);
        header.add_css_class(CLASS_HEADER);
        header.set_hexpand(true);

        let handle_label = Label::new(Some("\u{2261}"));
        handle_label.add_css_class(CLASS_HANDLE);
        handle_label.set_visible(false);
        header.append(&handle_label);

        let title_label = Label::new(None);
        title_label.set_hexpand(true);
        title_label.set_halign(Align::Start);
        title_label.set_ellipsize(EllipsizeMode::End);
        header.append(&title_label);

        let fold_button = Button::from_icon_name("go-down-symbolic");
        fold_button.add_css_class("flat");
        fold_button.set_visible(false);
        header.append(&fold_button);

        let close_button = Button::from_icon_name("window-close-symbolic");
        close_button.add_css_class("flat");
        close_button.set_visible(false);
        header.append(&close_button);

        let content_holder = gtk4::Box::new(Orientation::Vertical, 0);
        content_holder.add_css_class(CLASS_CONTENT);
        content_holder.set_vexpand(true);

        // The overlay stacks the resize grip over the content's
        // bottom-right corner without claiming layout space.
        let content_overlay = Overlay::new();
        content_overlay.set_child(Some(&content_holder));
        let resize_grip = resize_grip();
        content_overlay.add_overlay(&resize_grip);
        let rotate_grip = rotate_grip();
        content_overlay.add_overlay(&rotate_grip);

        let folded_holder = gtk4::Box::new(Orientation::Horizontal, 0);
        folded_holder.add_css_class(crate::widget::css::CLASS_FOLDED_CONTENT);
        folded_holder.set_visible(false);

        card.append(&header);
        card.append(&content_overlay);
        card.append(&folded_holder);

        let _ = self.header.set(header);
        let _ = self.content_holder.set(content_holder);
        let _ = self.title_label.set(title_label);
        let _ = self.handle_label.set(handle_label);
        let _ = self.fold_button.set(fold_button);
        let _ = self.close_button.set(close_button);
        let _ = self.resize_grip.set(resize_grip);
        let _ = self.rotate_grip.set(rotate_grip);
        let _ = self.folded_holder.set(folded_holder);
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: LazyLock<Vec<Signal>> = LazyLock::new(|| CARD_SIGNALS.iter().map(|signal| signal.signal()).collect());
        &SIGNALS
    }
}

/// Edge length of the resize grip's diagonal strokes (px).
const RESIZE_GRIP_SIZE: i32 = 18;

/// Builds the bottom-right resize affordance: three diagonal strokes in
/// the card's foreground color, `nwse-resize` cursor on hover.
fn resize_grip() -> DrawingArea {
    let grip = DrawingArea::new();
    grip.add_css_class(CLASS_RESIZE_GRIP);
    // Overlay children default to `Align::Fill` - without End
    // alignment the grip would cover the whole content area.
    grip.set_halign(Align::End);
    grip.set_valign(Align::End);
    grip.set_content_width(RESIZE_GRIP_SIZE);
    grip.set_content_height(RESIZE_GRIP_SIZE);
    grip.set_cursor_from_name(Some("nwse-resize"));
    grip.set_visible(false);
    grip.set_draw_func(|grip, cr, _, _| {
        let color = grip.color();
        let width = f64::from(grip.width());
        let height = f64::from(grip.height());
        cr.set_source_rgba(f64::from(color.red()), f64::from(color.green()), f64::from(color.blue()), f64::from(color.alpha()));
        cr.set_line_width(1.5);
        cr.set_line_cap(LineCap::Round);
        for inset in [4.0, 8.0, 12.0] {
            cr.move_to(width - inset, height - 2.0);
            cr.line_to(width - 2.0, height - inset);
            let _ = cr.stroke();
        }
    });
    grip
}

/// Edge length of the rotate grip's circular arrow (px).
const ROTATE_GRIP_SIZE: i32 = 18;

/// Builds the bottom-left rotate affordance: a circular arrow in the
/// card's foreground color, `grab` cursor on hover.
fn rotate_grip() -> DrawingArea {
    let grip = DrawingArea::new();
    grip.add_css_class(CLASS_ROTATE_GRIP);
    // Overlay children default to `Align::Fill` - without Start/End
    // alignment the grip would cover the whole content area.
    grip.set_halign(Align::Start);
    grip.set_valign(Align::End);
    grip.set_content_width(ROTATE_GRIP_SIZE);
    grip.set_content_height(ROTATE_GRIP_SIZE);
    grip.set_cursor_from_name(Some("grab"));
    grip.set_visible(false);
    grip.set_draw_func(|grip, cr, _, _| {
        let color = grip.color();
        let width = f64::from(grip.width());
        let height = f64::from(grip.height());
        cr.set_source_rgba(f64::from(color.red()), f64::from(color.green()), f64::from(color.blue()), f64::from(color.alpha()));
        cr.set_line_width(1.5);
        cr.set_line_cap(LineCap::Round);
        // A three-quarter clockwise arc with an arrowhead on its tip:
        // the circular arrow suggests turning in either direction.
        let radius = width.min(height) / 2.0 - 3.0;
        let center_x = width / 2.0;
        let center_y = height / 2.0;
        let start = -FRAC_PI_2 + 0.7;
        let end = start + 3.0 * FRAC_PI_2;
        cr.arc(center_x, center_y, radius, start, end);
        let _ = cr.stroke();
        let tip_x = center_x + radius * end.cos();
        let tip_y = center_y + radius * end.sin();
        let tangent = end + FRAC_PI_2;
        for spread in [-0.7, 0.7] {
            cr.move_to(tip_x, tip_y);
            cr.line_to(tip_x - 4.0 * (tangent + spread).cos(), tip_y - 4.0 * (tangent + spread).sin());
        }
        let _ = cr.stroke();
    });
    grip
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

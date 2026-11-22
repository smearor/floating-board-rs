//! Chrome element builders and mounting, implemented on [`Card`].
//!
//! One [`ChromeElement`] produces one widget plus its signal wiring
//! (P1.1). `Inline` elements mount either into the card's header strip
//! (drag handle, title, minimize, close) or onto the content overlay
//! (resize/rotate grips); `Satellite` placements are validated and
//! skipped until satellite mounting lands (Phase 2/3).
//!
//! The `builtin` renderer assembles the stock GTK4/CSS chrome; the
//! `adwaita` feature adds a libadwaita renderer (`adw::HeaderBar`,
//! `adw::WindowTitle`, wired buttons in a `windowcontrols` node). The
//! bare widget factories at the bottom of this module stay free
//! functions - they are stateless builders for foreign types (`Label`,
//! `DrawingArea`).

use crate::domain::ChromeElement;
use crate::domain::ChromePlacement;
use crate::domain::ChromeSlot;
use crate::domain::ChromeStyle;
use crate::l10n::l10n;
use crate::widget::card::Card;
use crate::widget::card_config::CardConfig;
use crate::widget::css::CLASS_HANDLE;
use crate::widget::css::CLASS_HEADER;
use crate::widget::css::CLASS_RESIZE_GRIP;
use crate::widget::css::CLASS_ROTATE_GRIP;
use glib::object::Cast;
use glib::object::ObjectExt;
use gtk4::Align;
use gtk4::Button;
use gtk4::DrawingArea;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::cairo::LineCap;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::BoxExt;
use gtk4::prelude::ButtonExt;
use gtk4::prelude::DrawingAreaExt;
use gtk4::prelude::DrawingAreaExtManual;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use std::f64::consts::FRAC_PI_2;
use tracing::debug;
use tracing::warn;

/// Icon the inline minimize slot shows while unfolded (toggles to
/// `go-up-symbolic` while folded - see `Card::set_folded`).
const ICON_MINIMIZE_INLINE: &str = "go-down-symbolic";

/// Adwaita-style minimize icon - used by `ChromeStyle::Adwaita` and,
/// later, by the satellite mount. The button keeps this icon
/// independent of fold state (`set_folded` only swaps `go-*` icons).
#[allow(dead_code)]
pub(crate) const ICON_MINIMIZE_SYMBOLIC: &str = "window-minimize-symbolic";

/// `EventController::name` tag marking gestures that belong to the
/// card's own wiring. The adwaita renderer strips the HeaderBar's
/// internal window gestures (move-drag, double-click maximize) but
/// spares controllers carrying this name.
pub(crate) const CARD_GESTURE_NAME: &str = "floating-board-card-gesture";

impl Card {
    /// Builds the card chrome from `config`'s effective slot list and
    /// mounts every `Inline` element: header elements onto a header
    /// strip prepended to the card, grip elements onto the content
    /// overlay.
    ///
    /// The header widget lands in `CardImpl::header` - it stays the
    /// drag affordance for `DragHandle::Header` and the strip that
    /// remains visible while folded. A header without any inline
    /// children is not mounted at all (an explicit `chrome: []` config
    /// produces a chrome-free card; use `DragHandle::WholeCard` or
    /// `headerless` for those).
    pub(crate) fn mount_chrome(&self, config: &CardConfig) {
        let title = config.title.clone().unwrap_or_else(|| l10n().message("card-untitled"));
        *self.imp().title_text.borrow_mut() = title.clone();
        let slots = config.effective_chrome_slots();
        let style = config.chrome_style.resolved();
        if style != config.chrome_style {
            warn!("CardConfig::chrome_style is Adwaita but the `adwaita` feature is disabled - falling back to Builtin");
        }
        match style {
            ChromeStyle::Builtin => self.mount_builtin_chrome(&title, &slots),
            #[cfg(feature = "adwaita")]
            ChromeStyle::Adwaita => self.mount_adwaita_chrome(config, &title, &slots),
            // `resolved()` already fell back to Builtin - unreachable in
            // practice, mount builtin defensively rather than panic.
            #[cfg(not(feature = "adwaita"))]
            ChromeStyle::Adwaita => self.mount_builtin_chrome(&title, &slots),
        }
        let imp = self.imp();
        // The grip gesture wiring in `FloatingBoard` keys off the grip's
        // existence, not the flags: a grip exists iff its slot was mounted.
        imp.resizable.set(imp.resize_grip.get().is_some());
        imp.rotate_grip_enabled.set(imp.rotate_grip.get().is_some());
    }

    /// `Builtin` renderer: header strip as a `gtk4::Box` of inline
    /// chrome slots, grips on the content overlay. Reproduces the
    /// historical card look one-to-one (P1.3 regression baseline).
    fn mount_builtin_chrome(&self, title: &str, slots: &[ChromeSlot]) {
        let imp = self.imp();
        let header = gtk4::Box::new(Orientation::Horizontal, 6);
        header.add_css_class(CLASS_HEADER);
        header.set_hexpand(true);
        let mut header_children = 0usize;
        for slot in slots {
            match (slot.element, slot.placement) {
                (ChromeElement::DragHandle, ChromePlacement::Inline) => {
                    let handle = drag_handle_label();
                    header.append(&handle);
                    let _ = imp.handle_label.set(handle);
                    header_children += 1;
                }
                (ChromeElement::Title, ChromePlacement::Inline) => {
                    let label = title_label(title);
                    header.append(&label);
                    let _ = imp.title_label.set(label);
                    header_children += 1;
                }
                (ChromeElement::Minimize, ChromePlacement::Inline) => {
                    let button = self.build_minimize_button(ICON_MINIMIZE_INLINE);
                    header.append(&button);
                    let _ = imp.fold_button.set(button);
                    header_children += 1;
                }
                (ChromeElement::Close, ChromePlacement::Inline) => {
                    let button = self.build_close_button();
                    header.append(&button);
                    let _ = imp.close_button.set(button);
                    header_children += 1;
                }
                (ChromeElement::ResizeGrip, ChromePlacement::Inline) => self.mount_grip(ChromeElement::ResizeGrip),
                (ChromeElement::RotateGrip, ChromePlacement::Inline) => self.mount_grip(ChromeElement::RotateGrip),
                (_, ChromePlacement::Satellite) => skip_satellite(slot.element),
            }
        }
        // A header without children is skipped entirely: no dead strip
        // is drawn and `drag_source(DragHandle::Header)` falls back to
        // the card widget, keeping a chrome-free card draggable.
        if header_children > 0 {
            imp.header_shown.set(true);
            self.prepend(&header);
            let _ = imp.header.set(header.upcast());
        }
    }

    /// Mounts the grip widget for `element` on the card's content
    /// overlay and records it in the matching `CardImpl` cell.
    /// Non-grip elements are ignored.
    fn mount_grip(&self, element: ChromeElement) {
        let imp = self.imp();
        let Some(overlay) = imp.content_overlay.get() else {
            return;
        };
        let (grip, cell) = match element {
            ChromeElement::ResizeGrip => (resize_grip(), &imp.resize_grip),
            ChromeElement::RotateGrip => (rotate_grip(), &imp.rotate_grip),
            _ => return,
        };
        overlay.add_overlay(&grip);
        let _ = cell.set(grip);
    }

    /// Mount-agnostic minimize button (P1.5): the same builder serves
    /// the inline header slot and, later, the satellite mount;
    /// `icon_name` differs per mount (`go-down-symbolic` inline,
    /// `window-minimize-symbolic` satellite). Wired to `Card::fold`,
    /// tooltip `card-fold`.
    pub(crate) fn build_minimize_button(&self, icon_name: &str) -> Button {
        let button = Button::from_icon_name(icon_name);
        button.add_css_class("flat");
        button.set_tooltip_text(Some(&l10n().message("card-fold")));
        let card_weak = self.downgrade();
        button.connect_clicked(move |_| {
            if let Some(card) = card_weak.upgrade() {
                card.fold();
            }
        });
        button
    }

    /// Mount-agnostic close button: inline header slot today, satellite
    /// mount later. Wired to `FloatingBoard::close_card` (the board and
    /// id come from the card's own backlink), tooltip `card-close`.
    pub(crate) fn build_close_button(&self) -> Button {
        let button = Button::from_icon_name("window-close-symbolic");
        button.add_css_class("flat");
        button.set_tooltip_text(Some(&l10n().message("card-close")));
        let board_weak = self.imp().board.borrow().clone();
        let id = self.id();
        button.connect_clicked(move |_| {
            if let Some(board) = board_weak.upgrade() {
                let _ = board.close_card(&id);
            }
        });
        button
    }
}

/// A `Satellite` slot: valid for satellite-capable elements but not yet
/// mounted (Phase 2/3); elements that can never be satellites warn.
fn skip_satellite(element: ChromeElement) {
    if element.supports_satellite() {
        debug!("Chrome element {element:?} configured as satellite - skipped until satellite mounting lands (Phase 2/3)");
    } else {
        warn!("Chrome element {element:?} does not support satellite placement - slot ignored");
    }
}

/// The `≡` drag affordance shown in `DragHandle::Handle` mode.
fn drag_handle_label() -> Label {
    let handle = Label::new(Some("\u{2261}"));
    handle.add_css_class(CLASS_HANDLE);
    handle.set_tooltip_text(Some(&l10n().message("card-handle")));
    handle
}

/// The header title: expanding label, start-aligned, ellipsized.
/// `title_text` in `CardImpl` stays the source of truth for
/// `Card::title` because the adwaita renderer swaps this for an
/// `adw::WindowTitle`.
fn title_label(title: &str) -> Label {
    let label = Label::new(Some(title));
    label.set_hexpand(true);
    label.set_halign(Align::Start);
    label.set_ellipsize(EllipsizeMode::End);
    label
}

/// Edge length of the resize grip's diagonal strokes (px).
const RESIZE_GRIP_SIZE: i32 = 18;

/// The bottom-right resize affordance: three diagonal strokes in the
/// card's foreground color, `nwse-resize` cursor on hover.
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
    grip.set_tooltip_text(Some(&l10n().message("card-resize")));
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

/// The bottom-left rotate affordance: a circular arrow in the card's
/// foreground color, `grab` cursor on hover.
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
    grip.set_tooltip_text(Some(&l10n().message("card-rotate")));
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

/// `Adwaita` renderer (feature `adwaita`): libadwaita chrome per P1.4 -
/// `adw::HeaderBar` as the header strip (with `default-decoration`, the
/// slim window-titlebar variant), `adw::WindowTitle` for the `Title`
/// element. `Minimize`/`Close` render as real wired buttons (the same
/// builders the builtin renderer uses) inside `windowcontrols`-named
/// containers placed by `gtk-decoration-layout`, so the libadwaita
/// stylesheet and GTK themes (e.g. Rewaita) treat them exactly like
/// window controls; `DragHandle` and the grips stay custom widgets.
///
/// Requires `adw::init()` (or an `adw::Application`) in the consumer -
/// libadwaita warns otherwise. A configured `CardPalette` keeps
/// precedence over the Adwaita theme through the scoped CSS installed
/// by `install_palette`; the widget structure stays Adwaita.
#[cfg(feature = "adwaita")]
mod adwaita {
    use crate::domain::ChromeElement;
    use crate::domain::ChromePlacement;
    use crate::domain::ChromeSlot;
    use crate::widget::card::Card;
    use crate::widget::card_config::CardConfig;
    use crate::widget::chrome::CARD_GESTURE_NAME;
    use crate::widget::chrome::ICON_MINIMIZE_SYMBOLIC;
    use crate::widget::chrome::drag_handle_label;
    use crate::widget::chrome::skip_satellite;
    use crate::widget::css::CLASS_HEADER;
    use glib::object::Cast;
    use glib::object::IsA;
    use glib::prelude::ObjectExt;
    use gtk4::gio::prelude::ListModelExt;
    use gtk4::prelude::BoxExt;
    use gtk4::prelude::EventControllerExt;
    use gtk4::prelude::WidgetExt;
    use gtk4::subclass::prelude::ObjectSubclassIsExt;
    use libadwaita as adw;
    use tracing::debug;

    pub mod imp {
        use glib::subclass::prelude::ObjectSubclass;
        use gtk4::subclass::prelude::BoxImpl;
        use gtk4::subclass::prelude::ObjectImpl;
        use gtk4::subclass::prelude::WidgetClassExt;
        use gtk4::subclass::prelude::WidgetImpl;

        /// A `gtk4::Box` whose CSS node name is `windowcontrols`, so the
        /// libadwaita stylesheet renders its buttons exactly like the
        /// window controls of a real titlebar (sizing, circular image
        /// hover) - the node selector cannot be faked with a CSS class.
        #[derive(Default)]
        pub struct ChromeControls;

        #[glib::object_subclass]
        impl ObjectSubclass for ChromeControls {
            const NAME: &'static str = "FloatingBoardChromeControls";
            type Type = super::ChromeControls;
            type ParentType = gtk4::Box;

            fn class_init(klass: &mut Self::Class) {
                klass.set_css_name("windowcontrols");
            }
        }

        impl ObjectImpl for ChromeControls {}
        impl WidgetImpl for ChromeControls {}
        impl BoxImpl for ChromeControls {}
    }

    glib::wrapper! {
        /// `gtk4::Box` carrying the `windowcontrols` CSS node name - see
        /// [`imp::ChromeControls`].
        pub struct ChromeControls(ObjectSubclass<imp::ChromeControls>)
            @extends gtk4::Box, gtk4::Widget,
            @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
    }

    impl ChromeControls {
        /// An empty controls container; libadwaita's `border-spacing`
        /// rule spaces the children.
        fn new() -> Self {
            glib::Object::new()
        }
    }

    impl Card {
        /// Builds the `adw::HeaderBar` chrome for the card.
        pub(super) fn mount_adwaita_chrome(&self, config: &CardConfig, title: &str, slots: &[ChromeSlot]) {
            let imp = self.imp();
            if !adw::is_initialized() {
                debug!("ChromeStyle::Adwaita configured - call `adw::init()` (or use `adw::Application`) for themed styling");
            }
            let header = adw::HeaderBar::new();
            header.set_show_start_title_buttons(false);
            header.set_show_end_title_buttons(false);
            // The headerbar paints the window chrome background; keep the
            // card's header CSS class so the palette/theme keeps styling
            // the frame around it.
            header.add_css_class(CLASS_HEADER);
            // Real Adwaita *window* titlebars carry `default-decoration`:
            // the stylesheet shrinks them to 37px with 22px controls. A
            // plain `adw::HeaderBar` would render at the 46px app-header
            // height - noticeably taller than both our builtin header and
            // a real window titlebar.
            header.add_css_class("default-decoration");
            // The HeaderBar registers internal gestures that drive the
            // *toplevel window* (move-drag, double-click maximize). On a
            // card they race the card's own drag - a fast drag moved the
            // whole window instead of the card. Watch for them from the
            // start, then strip the subtree after our children exist.
            let watcher = header.observe_controllers();
            watch_foreign_gestures(&header, &watcher);
            // The watcher only covers the header's *own* controller
            // list; libadwaita can also register gestures lazily on
            // inner widgets (windowhandle) at map time. Strip the whole
            // subtree once more when the header is actually shown.
            header.connect_map(strip_foreign_gestures);
            // Wired control buttons, collected per slot and placed by
            // `pack_window_controls` afterwards: side and order follow
            // the `gtk-decoration-layout` setting (themes like Rewaita
            // move window buttons to the left).
            let mut minimize_button: Option<gtk4::Button> = None;
            let mut close_button: Option<gtk4::Button> = None;
            let mut header_children = 0usize;
            for slot in slots {
                match (slot.element, slot.placement) {
                    (ChromeElement::DragHandle, ChromePlacement::Inline) => {
                        let handle = drag_handle_label();
                        header.pack_start(&handle);
                        let _ = imp.handle_label.set(handle);
                        header_children += 1;
                    }
                    (ChromeElement::Title, ChromePlacement::Inline) => {
                        let title_widget = adw::WindowTitle::new(title, "");
                        title_widget.set_visible(!config.headerless);
                        header.set_title_widget(Some(&title_widget));
                        header_children += 1;
                    }
                    (ChromeElement::Minimize, ChromePlacement::Inline) => {
                        // A real wired button: `WindowControls` buttons
                        // are decorative (they target a real toplevel).
                        let button = self.build_minimize_button(ICON_MINIMIZE_SYMBOLIC);
                        // Let the `windowcontrols` node rules own the
                        // look (the `flat` class would fight their
                        // padding/hover styling).
                        button.remove_css_class("flat");
                        // Real window controls carry their role as a CSS
                        // class - themes (e.g. Rewaita) key button colors
                        // off `windowcontrols button.minimize`.
                        button.add_css_class("minimize");
                        let _ = imp.fold_button.set(button.clone());
                        minimize_button = Some(button);
                        header_children += 1;
                    }
                    (ChromeElement::Close, ChromePlacement::Inline) => {
                        let button = self.build_close_button();
                        button.remove_css_class("flat");
                        button.add_css_class("close");
                        let _ = imp.close_button.set(button.clone());
                        close_button = Some(button);
                        header_children += 1;
                    }
                    (ChromeElement::ResizeGrip, ChromePlacement::Inline) => self.mount_grip(ChromeElement::ResizeGrip),
                    (ChromeElement::RotateGrip, ChromePlacement::Inline) => self.mount_grip(ChromeElement::RotateGrip),
                    (_, ChromePlacement::Satellite) => skip_satellite(slot.element),
                }
            }
            pack_window_controls(&header, minimize_button, close_button);
            strip_foreign_gestures(&header);
            // Same rule as the builtin renderer: no children, no header -
            // `drag_source(DragHandle::Header)` falls back to the card.
            if header_children > 0 {
                imp.header_shown.set(true);
                self.prepend(&header);
                let _ = imp.header.set(header.upcast());
            }
        }
    }

    /// Fallback when `gtk-decoration-layout` is unset: all controls on
    /// the right in GNOME order.
    const DEFAULT_DECORATION_LAYOUT: &str = ":minimize,maximize,close";

    /// Splits `gtk-decoration-layout` into the button names left and
    /// right of the `:` separator (e.g. Rewaita's `"close,minimize:"`
    /// moves all controls to the left side).
    fn decoration_layout_names() -> (Vec<String>, Vec<String>) {
        let layout = gtk4::Settings::default()
            .and_then(|settings| settings.gtk_decoration_layout())
            .map(|layout| layout.to_string())
            .unwrap_or_else(|| DEFAULT_DECORATION_LAYOUT.to_string());
        let names = |side: &str| side.split(',').map(str::trim).filter(|name| !name.is_empty()).map(str::to_string).collect();
        let mut sides = layout.splitn(2, ':');
        let start = names(sides.next().unwrap_or_default());
        let end = names(sides.next().unwrap_or_default());
        (start, end)
    }

    /// Packs the card's control buttons into `windowcontrols` containers,
    /// following the decoration layout: a button goes to its named side in
    /// layout order; slotted buttons the layout does not mention append to
    /// the end side (a configured slot is never dropped, only reordered).
    fn pack_window_controls(header: &adw::HeaderBar, minimize: Option<gtk4::Button>, close: Option<gtk4::Button>) {
        let (start_names, end_names) = decoration_layout_names();
        let start_controls = ChromeControls::new();
        let end_controls = ChromeControls::new();
        let mut minimize = minimize;
        let mut close = close;
        let mut take = |name: &str| match name {
            "minimize" => minimize.take(),
            "close" => close.take(),
            _ => None,
        };
        for name in &start_names {
            if let Some(button) = take(name) {
                start_controls.append(&button);
            }
        }
        for name in &end_names {
            if let Some(button) = take(name) {
                end_controls.append(&button);
            }
        }
        drop(take);
        if let Some(button) = minimize.take() {
            end_controls.append(&button);
        }
        if let Some(button) = close.take() {
            end_controls.append(&button);
        }
        if start_controls.first_child().is_some() {
            // `start`/`end` mirror what GtkWindowControls' own boxes
            // carry; idempotent where the headerbar already adds them.
            start_controls.add_css_class("start");
            header.pack_start(&start_controls);
        }
        if end_controls.first_child().is_some() {
            end_controls.add_css_class("end");
            header.pack_end(&end_controls);
        }
    }

    /// A gesture controller not tagged `CARD_GESTURE_NAME` - i.e. one of
    /// the HeaderBar's internal window gestures.
    fn is_foreign_gesture(controller: &gtk4::EventController) -> bool {
        controller.is::<gtk4::Gesture>() && controller.name().as_deref() != Some(CARD_GESTURE_NAME)
    }

    /// Removes every foreign gesture on `widget` and its descendants,
    /// recursing into children but skipping `gtk4::Button` subtrees
    /// *before* touching controllers - a button's internal `GestureClick`
    /// is a foreign gesture by the tag rule, yet it is exactly what
    /// makes the button work.
    fn strip_foreign_gestures(widget: &impl IsA<gtk4::Widget>) {
        let widget = widget.upcast_ref::<gtk4::Widget>();
        if widget.is::<gtk4::Button>() {
            return;
        }
        let controllers = widget.observe_controllers();
        for index in (0..controllers.n_items()).rev() {
            let Some(item) = controllers.item(index) else { continue };
            let Ok(controller) = item.downcast::<gtk4::EventController>() else {
                continue;
            };
            if is_foreign_gesture(&controller) {
                widget.remove_controller(&controller);
            }
        }
        let mut child = widget.first_child();
        while let Some(sub) = child {
            strip_foreign_gestures(&sub);
            child = sub.next_sibling();
        }
    }

    /// Watches the header's controller list for foreign gestures
    /// registered *after* construction (e.g. on realize) and removes
    /// them. Removal is deferred out of the `items_changed` emission -
    /// mutating a list model inside its own change handler is fragile.
    fn watch_foreign_gestures(header: &adw::HeaderBar, controllers: &gtk4::gio::ListModel) {
        let header_weak = header.downgrade();
        controllers.connect_items_changed(move |model, position, _removed, added| {
            for index in position..position + added {
                let Some(item) = model.item(index) else { continue };
                let Ok(controller) = item.downcast::<gtk4::EventController>() else {
                    continue;
                };
                if !is_foreign_gesture(&controller) {
                    continue;
                }
                let Some(header) = header_weak.upgrade() else { continue };
                gtk4::glib::idle_add_local_once(move || header.remove_controller(&controller));
            }
        });
    }
}

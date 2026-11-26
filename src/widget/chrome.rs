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

use crate::domain::CardShape;
use crate::domain::ChromeElement;
use crate::domain::ChromePlacement;
use crate::domain::ChromeSlot;
use crate::domain::ChromeStyle;
use crate::domain::TurnDirection;
use crate::l10n::l10n;
use crate::widget::card::Card;
use crate::widget::card_config::CardConfig;
use crate::widget::css::CLASS_ARC_BAND;
use crate::widget::css::CLASS_ARC_CHROME;
use crate::widget::css::CLASS_HANDLE;
use crate::widget::css::CLASS_HEADER;
use crate::widget::css::CLASS_RESIZE_GRIP;
use crate::widget::css::CLASS_ROTATE_GRIP;
use crate::widget::imp::card::ARC_CHROME_BAND;
use glib::object::Cast;
use glib::object::ObjectExt;
use gtk4::Align;
use gtk4::Button;
use gtk4::DrawingArea;
use gtk4::Label;
use gtk4::Orientation;
use gtk4::Widget;
use gtk4::cairo::LineCap;
use gtk4::pango::EllipsizeMode;
use gtk4::prelude::BoxExt;
use gtk4::prelude::ButtonExt;
use gtk4::prelude::DrawingAreaExt;
use gtk4::prelude::DrawingAreaExtManual;
use gtk4::prelude::WidgetExt;
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use smearor_commons_geometry::Degrees;
use smearor_commons_geometry::RotationDirection;
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
        // A circle card has no header strip: its inline chrome mounts
        // along the disc's top arc as an "arc header" instead;
        // `Satellite` placements stay satellites.
        if config.shape == CardShape::Circle {
            self.mount_arc_chrome(config);
            return;
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

    /// `Circle` renderer: inline chrome slots along the disc's top arc
    /// (`CardConfig::arc_span`, default a quarter) as an "arc header" -
    /// `CardImpl::circle_allocate` places them on the band's mid
    /// radius, a `Title` pinned to the arc's center like a header
    /// bar's. `Satellite` slots stay satellites; the edge grips have
    /// no mount on a circle.
    fn mount_arc_chrome(&self, config: &CardConfig) {
        let imp = self.imp();
        let span = config
            .arc_span
            .unwrap_or(Degrees::QUARTER_TURN)
            .as_f64()
            .clamp(Degrees::ZERO.as_f64(), Degrees::FULL_TURN.as_f64());
        imp.arc_span.set(Degrees::new(span).to_radians());
        let title = imp.title_text.borrow().clone();
        let slots = config.effective_chrome_slots();
        let style = config.chrome_style.resolved();
        let mut items = Vec::new();
        for slot in &slots {
            if slot.placement == ChromePlacement::Satellite {
                skip_satellite(slot.element);
                continue;
            }
            let widget: Widget = match slot.element {
                ChromeElement::DragHandle => {
                    let handle = drag_handle_label();
                    let _ = imp.handle_label.set(handle.clone());
                    handle.upcast()
                }
                ChromeElement::Title => {
                    let label = title_label(&title);
                    // The arc's centered item reads like a header bar's
                    // title - it must not claim inline space.
                    label.set_hexpand(false);
                    label.set_halign(Align::Center);
                    let _ = imp.title_label.set(label.clone());
                    label.upcast()
                }
                ChromeElement::Minimize => {
                    let button = self.build_minimize_button(ICON_MINIMIZE_INLINE);
                    let _ = imp.fold_button.set(button.clone());
                    button.upcast()
                }
                ChromeElement::Close => {
                    let button = self.build_close_button();
                    let _ = imp.close_button.set(button.clone());
                    button.upcast()
                }
                ChromeElement::RotateClockwise => self.build_rotate_button(RotationDirection::Clockwise).upcast(),
                ChromeElement::RotateCounterClockwise => self.build_rotate_button(RotationDirection::CounterClockwise).upcast(),
                ChromeElement::NextSeat => self.build_pass_button(TurnDirection::Clockwise).upcast(),
                ChromeElement::PreviousSeat => self.build_pass_button(TurnDirection::CounterClockwise).upcast(),
                ChromeElement::ResizeGrip | ChromeElement::RotateGrip => {
                    warn!(element = ?slot.element, "edge grip has no mount on a circle card - skipped");
                    continue;
                }
            };
            widget.add_css_class(CLASS_ARC_CHROME);
            if style == ChromeStyle::Adwaita {
                // The themed look chrome satellites get: a raised round
                // button instead of the builtin flat icon.
                widget.remove_css_class("flat");
                widget.add_css_class("circular");
            }
            self.append(&widget);
            items.push((widget, slot.element));
        }
        // The painted band sits beneath the items (earlier children
        // paint first) and is created only when items exist - an
        // empty band would be a stray ring sector.
        if !items.is_empty() {
            let band = arc_band(self);
            band.insert_before(self, items.first().map(|(first, _)| first));
            let _ = imp.arc_band.set(band);
        }
        *imp.arc_items.borrow_mut() = items;
    }

    /// `Builtin` renderer: header strip as a `gtk4::Box` of inline
    /// chrome slots, grips on the content overlay. Reproduces the
    /// historical card look one-to-one (P1.3 regression baseline).
    fn mount_builtin_chrome(&self, title: &str, slots: &[ChromeSlot]) {
        let imp = self.imp();
        let header = gtk4::Box::new(Orientation::Horizontal, 6);
        header.add_css_class(CLASS_HEADER);
        header.set_hexpand(true);
        // Header children are grouped by placement: `InlineStart`
        // children land before the neutral `Inline` run (which keeps
        // slot order), `InlineEnd` children after it - so ends of the
        // strip are controllable independent of slot order.
        let mut start_children: Vec<Widget> = Vec::new();
        let mut middle_children: Vec<Widget> = Vec::new();
        let mut end_children: Vec<Widget> = Vec::new();
        for slot in slots {
            if slot.placement == ChromePlacement::Satellite {
                skip_satellite(slot.element);
                continue;
            }
            let target = match slot.placement {
                ChromePlacement::InlineStart => &mut start_children,
                ChromePlacement::InlineEnd => &mut end_children,
                _ => &mut middle_children,
            };
            match slot.element {
                ChromeElement::DragHandle => {
                    let handle = drag_handle_label();
                    target.push(handle.clone().upcast());
                    let _ = imp.handle_label.set(handle);
                }
                ChromeElement::Title => {
                    let label = title_label(title);
                    target.push(label.clone().upcast());
                    let _ = imp.title_label.set(label);
                }
                ChromeElement::Minimize => {
                    let button = self.build_minimize_button(ICON_MINIMIZE_INLINE);
                    target.push(button.clone().upcast());
                    let _ = imp.fold_button.set(button);
                }
                ChromeElement::Close => {
                    let button = self.build_close_button();
                    target.push(button.clone().upcast());
                    let _ = imp.close_button.set(button);
                }
                ChromeElement::RotateClockwise => target.push(self.build_rotate_button(RotationDirection::Clockwise).upcast()),
                ChromeElement::RotateCounterClockwise => target.push(self.build_rotate_button(RotationDirection::CounterClockwise).upcast()),
                ChromeElement::NextSeat => target.push(self.build_pass_button(TurnDirection::Clockwise).upcast()),
                ChromeElement::PreviousSeat => target.push(self.build_pass_button(TurnDirection::CounterClockwise).upcast()),
                ChromeElement::ResizeGrip => self.mount_grip(ChromeElement::ResizeGrip),
                ChromeElement::RotateGrip => self.mount_grip(ChromeElement::RotateGrip),
            }
        }
        let header_children = start_children.len() + middle_children.len() + end_children.len();
        for child in start_children.iter().chain(&middle_children).chain(&end_children) {
            header.append(child);
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

    /// Mount-agnostic rotate button stepping the card one snap step in
    /// `direction` per click (`Card::rotate_to_next_quarter`) - the
    /// shared builder for `ChromeElement::RotateClockwise` and
    /// `RotateCounterClockwise`, inline and satellite mounts alike.
    pub(crate) fn build_rotate_button(&self, direction: RotationDirection) -> Button {
        let (icon_name, tooltip) = match direction {
            RotationDirection::Clockwise => ("object-rotate-right-symbolic", "card-rotate-clockwise"),
            RotationDirection::CounterClockwise => ("object-rotate-left-symbolic", "card-rotate-counter-clockwise"),
        };
        let button = Button::from_icon_name(icon_name);
        button.add_css_class("flat");
        button.set_tooltip_text(Some(&l10n().message(tooltip)));
        let card_weak = self.downgrade();
        button.connect_clicked(move |_| {
            if let Some(card) = card_weak.upgrade() {
                card.rotate_to_next_quarter(direction);
            }
        });
        button
    }

    /// Mount-agnostic pass button handing the card one table side in
    /// `direction` per click (`Card::pass_to_next_user`) - the shared
    /// builder for `ChromeElement::NextSeat` and `PreviousSeat`, inline
    /// and satellite mounts alike. `move_to_edge` is fixed to `true`:
    /// a passed card lands at the seat's edge lane.
    pub(crate) fn build_pass_button(&self, direction: TurnDirection) -> Button {
        let (icon_name, tooltip) = match direction {
            TurnDirection::Clockwise => ("go-previous-symbolic", "card-next-seat"),
            TurnDirection::CounterClockwise => ("go-next-symbolic", "card-previous-seat"),
        };
        let button = Button::from_icon_name(icon_name);
        button.add_css_class("flat");
        button.set_tooltip_text(Some(&l10n().message(tooltip)));
        let card_weak = self.downgrade();
        button.connect_clicked(move |_| {
            if let Some(card) = card_weak.upgrade() {
                let _ = card.pass_to_next_user(direction, true);
            }
        });
        button
    }
}

/// A `Satellite` slot never mounts inline - satellite-capable elements
/// are picked up by the board's satellite mounting (`CardConfig::
/// effective_satellites`); elements that can never be satellites warn.
fn skip_satellite(element: ChromeElement) {
    if element.supports_satellite() {
        debug!("Chrome element {element:?} configured as satellite - mounted by the board, not inline");
    } else {
        warn!("Chrome element {element:?} does not support satellite placement - slot ignored");
    }
}

/// The painted top-quarter annulus band behind a circle card's arc
/// chrome (`ARC_CHROME_BAND` thick, ±45°): a `DrawingArea` stroking a
/// round-capped arc on the band's mid radius in the card's header
/// color - read through the CSS `color` property so palettes tint it.
/// Input-transparent: the band is decoration, the ring stays a drag
/// surface.
fn arc_band(card: &Card) -> DrawingArea {
    let band = DrawingArea::new();
    band.add_css_class(CLASS_ARC_BAND);
    band.set_can_target(false);
    let card_weak = card.downgrade();
    band.set_draw_func(move |area, cr, width, height| {
        let radius = f64::from(width.min(height)) / 2.0 - ARC_CHROME_BAND / 2.0;
        let Some(card) = card_weak.upgrade() else { return };
        if radius <= 0.0 {
            return;
        }
        let half_span = card.imp().arc_span.get().as_f64() / 2.0;
        let color = area.color();
        cr.set_source_rgba(f64::from(color.red()), f64::from(color.green()), f64::from(color.blue()), f64::from(color.alpha()));
        cr.set_line_width(ARC_CHROME_BAND);
        cr.set_line_cap(LineCap::Round);
        cr.arc(
            f64::from(width) / 2.0,
            f64::from(height) / 2.0,
            radius,
            -std::f64::consts::FRAC_PI_2 - half_span,
            -std::f64::consts::FRAC_PI_2 + half_span,
        );
        let _ = cr.stroke();
    });
    band
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
    use crate::domain::TurnDirection;
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
    use smearor_commons_geometry::RotationDirection;
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
            // Control buttons route by placement: `InlineStart`
            // collects into the start `windowcontrols` container,
            // `InlineEnd` into the end container, neutral `Inline`
            // keeps the historical behavior - `Minimize`/`Close`
            // follow `gtk-decoration-layout`, other actions append to
            // the end controls.
            let mut minimize_button: Option<gtk4::Button> = None;
            let mut close_button: Option<gtk4::Button> = None;
            let mut start_extras: Vec<gtk4::Button> = Vec::new();
            let mut end_extras: Vec<gtk4::Button> = Vec::new();
            let mut header_children = 0usize;
            for slot in slots {
                if slot.placement == ChromePlacement::Satellite {
                    skip_satellite(slot.element);
                    continue;
                }
                match slot.element {
                    ChromeElement::DragHandle => {
                        let handle = drag_handle_label();
                        match slot.placement {
                            ChromePlacement::InlineEnd => header.pack_end(&handle),
                            _ => header.pack_start(&handle),
                        }
                        let _ = imp.handle_label.set(handle);
                        header_children += 1;
                    }
                    ChromeElement::Title => {
                        let title_widget = adw::WindowTitle::new(title, "");
                        title_widget.set_visible(!config.headerless);
                        match slot.placement {
                            ChromePlacement::InlineStart => header.pack_start(&title_widget),
                            ChromePlacement::InlineEnd => header.pack_end(&title_widget),
                            _ => header.set_title_widget(Some(&title_widget)),
                        }
                        header_children += 1;
                    }
                    _ => {
                        let (button, class) = match slot.element {
                            ChromeElement::Minimize => (self.build_minimize_button(ICON_MINIMIZE_SYMBOLIC), "minimize"),
                            ChromeElement::Close => (self.build_close_button(), "close"),
                            ChromeElement::RotateClockwise => (self.build_rotate_button(RotationDirection::Clockwise), "rotate-clockwise"),
                            ChromeElement::RotateCounterClockwise => {
                                (self.build_rotate_button(RotationDirection::CounterClockwise), "rotate-counter-clockwise")
                            }
                            ChromeElement::NextSeat => (self.build_pass_button(TurnDirection::Clockwise), "next-seat"),
                            ChromeElement::PreviousSeat => (self.build_pass_button(TurnDirection::CounterClockwise), "previous-seat"),
                            ChromeElement::ResizeGrip => {
                                self.mount_grip(ChromeElement::ResizeGrip);
                                continue;
                            }
                            ChromeElement::RotateGrip => {
                                self.mount_grip(ChromeElement::RotateGrip);
                                continue;
                            }
                            ChromeElement::DragHandle | ChromeElement::Title => continue,
                        };
                        // A real wired button: `WindowControls` buttons
                        // are decorative (they target a real toplevel).
                        // Let the `windowcontrols` node rules own the
                        // look (the `flat` class would fight their
                        // padding/hover styling), and carry the role
                        // class themes key colors off.
                        button.remove_css_class("flat");
                        button.add_css_class(class);
                        match slot.placement {
                            ChromePlacement::InlineStart => start_extras.push(button.clone()),
                            ChromePlacement::InlineEnd => end_extras.push(button.clone()),
                            _ => match slot.element {
                                ChromeElement::Minimize => minimize_button = Some(button.clone()),
                                ChromeElement::Close => close_button = Some(button.clone()),
                                _ => end_extras.push(button.clone()),
                            },
                        }
                        if slot.element == ChromeElement::Minimize {
                            let _ = imp.fold_button.set(button.clone());
                        }
                        if slot.element == ChromeElement::Close {
                            let _ = imp.close_button.set(button);
                        }
                        header_children += 1;
                    }
                }
            }
            pack_window_controls(&header, minimize_button, close_button, start_extras, end_extras);
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
    /// `extras` are card actions outside the window-button vocabulary
    /// (rotate/pass steps): `start_extras` lead the start controls
    /// (outermost first, like GNOME's app-buttons-left convention),
    /// `end_extras` trail the end controls.
    fn pack_window_controls(
        header: &adw::HeaderBar,
        minimize: Option<gtk4::Button>,
        close: Option<gtk4::Button>,
        start_extras: Vec<gtk4::Button>,
        end_extras: Vec<gtk4::Button>,
    ) {
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
        for extra in &start_extras {
            start_controls.append(extra);
        }
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
        for extra in &end_extras {
            end_controls.append(extra);
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

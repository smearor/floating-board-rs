use crate::domain::CardColor;
use crate::domain::CardPalette;
use glib::object::IsA;
use gtk4::CssProvider;
use gtk4::Widget;
use gtk4::gdk::Display;
use gtk4::prelude::WidgetExt;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

/// CSS class of the board widget itself.
pub const CLASS_BOARD: &str = "floating-board";
/// CSS class of the transparent card layer above the content.
pub const CLASS_CARD_LAYER: &str = "floating-card-layer";
/// CSS class of every card.
pub const CLASS_CARD: &str = "floating-card";
/// CSS class of a card's header strip (drag affordance).
pub const CLASS_HEADER: &str = "floating-card-header";
/// CSS class applied to cards configured `headerless`.
pub const CLASS_HEADERLESS: &str = "floating-card-headerless";
/// CSS class of the card content area wrapping the consumer widget.
pub const CLASS_CONTENT: &str = "floating-card-content";
/// CSS class of the dedicated drag handle in `DragHandle::Handle` mode.
pub const CLASS_HANDLE: &str = "floating-card-handle";
/// CSS class applied while a card is folded.
pub const CLASS_FOLDED: &str = "floating-card-folded";

/// Added alongside [`CLASS_FOLDED`] when the card folds into
/// consumer-supplied `CardConfig::folded_content` instead of the
/// header strip: the card chrome becomes transparent so the folded
/// widget (a round button, a compact toolbar, ...) defines the look.
pub const CLASS_COLLAPSED: &str = "floating-card-collapsed";

/// Wrapper around `CardConfig::folded_content` while the card is
/// folded; style custom collapsed looks against it.
pub const CLASS_FOLDED_CONTENT: &str = "floating-card-folded-content";
/// CSS class of every satellite widget.
pub const CLASS_SATELLITE: &str = "floating-card-satellite";
/// Additional CSS class of a chrome-element satellite.
pub const CLASS_SATELLITE_CHROME: &str = "floating-card-satellite-chrome";
/// Additional CSS class of a consumer-widget satellite.
pub const CLASS_SATELLITE_CUSTOM: &str = "floating-card-satellite-custom";
/// CSS class of the bottom-right resize grip (`CardConfig::resizable`).
pub const CLASS_RESIZE_GRIP: &str = "floating-card-resize-grip";
/// CSS class of the bottom-left rotate grip (`CardConfig::rotate_grip`).
pub const CLASS_ROTATE_GRIP: &str = "floating-card-rotate-grip";

/// CSS class of the board's corner view-control button box
/// (`FloatingBoard::set_view_buttons`).
pub const CLASS_VIEW_CONTROLS: &str = "floating-board-view-controls";

/// Sequence number suffixing the per-card palette class, so two
/// palette cards never share a selector.
static PALETTE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Attaches `palette` to `widget` via a generated CSS class plus a
/// display-scoped provider: the rules apply to the widget's subtree
/// only - the non-deprecated counterpart of a widget-scoped provider -
/// so cards can carry individual color schemes.
/// `STYLE_PROVIDER_PRIORITY_APPLICATION` beats the default stylesheet
/// and themes while user-level CSS still wins. `None` (and empty)
/// palettes install nothing.
pub(crate) fn install_palette(widget: &impl IsA<Widget>, palette: &CardPalette) {
    if palette.is_empty() {
        return;
    }
    let Some(display) = Display::default() else {
        return;
    };
    let class = format!("floating-card-palette-{}", PALETTE_SEQUENCE.fetch_add(1, Ordering::Relaxed));
    let provider = CssProvider::new();
    provider.load_from_string(&palette_stylesheet(palette, &class));
    gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION);
    widget.add_css_class(&class);
}

/// One CSS rule per filled [`CardColor`] slot, scoped to `class`.
fn palette_stylesheet(palette: &CardPalette, class: &str) -> String {
    let mut stylesheet = String::new();
    if let Some(background) = palette.get(CardColor::Background) {
        stylesheet.push_str(&format!(".{class} {{ background-color: {background}; }}\n"));
    }
    if let Some(foreground) = palette.get(CardColor::Foreground) {
        stylesheet.push_str(&format!(".{class} {{ color: {foreground}; }}\n"));
    }
    if let Some(header) = palette.get(CardColor::Header) {
        stylesheet.push_str(&format!(".{class} .{CLASS_HEADER} {{ background-color: {header}; }}\n"));
    }
    stylesheet
}

const DEFAULT_CSS: &str = include_str!("default.css");

static CSS_LOADED: AtomicBool = AtomicBool::new(false);

/// Installs the crate's default stylesheet on the default display.
///
/// The provider is registered with `STYLE_PROVIDER_PRIORITY_FALLBACK`, so
/// consumer themes and additional providers always win over these defaults.
/// Called lazily from `FloatingBoard` construction - before a display exists
/// the registration is skipped and retried on the next board.
pub(crate) fn ensure_default_css() {
    if CSS_LOADED.load(Ordering::Acquire) {
        return;
    }
    let Some(display) = Display::default() else {
        return;
    };
    let provider = CssProvider::new();
    provider.load_from_string(DEFAULT_CSS);
    gtk4::style_context_add_provider_for_display(&display, &provider, gtk4::STYLE_PROVIDER_PRIORITY_FALLBACK);
    CSS_LOADED.store(true, Ordering::Release);
}

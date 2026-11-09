use gtk4::CssProvider;
use gtk4::gdk::Display;
use std::sync::atomic::AtomicBool;
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

const DEFAULT_CSS: &str = "
.floating-card {
    background-color: #2e3440;
    color: #eceff4;
    border-radius: 12px;
    box-shadow: 0 6px 20px rgba(0, 0, 0, 0.45);
}
.floating-card-header {
    background-color: #3b4252;
    border-radius: 12px 12px 0 0;
    padding: 4px 8px;
    min-height: 26px;
}
.floating-card-folded .floating-card-header {
    border-radius: 12px;
}
.floating-card-headerless .floating-card-header {
    min-height: 10px;
    padding: 2px 8px;
}
.floating-card-content {
    padding: 10px;
}
.floating-card-handle {
    margin-right: 6px;
    opacity: 0.7;
}
";

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

use crate::widget::card::Card;
use glib::subclass::Signal;
use glib::types::StaticType;

/// The signals `FloatingBoard` emits, as a type-safe alternative to
/// stringly-typed signal names. Used by `signals()` in the widget
/// implementation and by every `emit_by_name` call site so each name lives
/// in exactly one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BoardSignal {
    /// Emitted with the `Card` after a card was added to the board.
    CardAdded,
    /// Emitted with the `Card` after a card was removed from the board.
    CardRemoved,
    /// Emitted with the `Card` when `set_card_visible` toggles visibility.
    CardVisibilityChanged,
    /// Emitted when any card's position, rotation, scale or z-order changed.
    GeometryChanged,
}

/// Every signal of the board, for `ObjectImpl::signals()`.
pub(crate) const BOARD_SIGNALS: &[BoardSignal] = &[
    BoardSignal::CardAdded,
    BoardSignal::CardRemoved,
    BoardSignal::CardVisibilityChanged,
    BoardSignal::GeometryChanged,
];

impl BoardSignal {
    /// The static GTK signal name (`"card-added"`).
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::CardAdded => "card-added",
            Self::CardRemoved => "card-removed",
            Self::CardVisibilityChanged => "card-visibility-changed",
            Self::GeometryChanged => "geometry-changed",
        }
    }

    /// Builds the GTK signal declaration with its parameter types - the
    /// card lifecycle signals carry the affected `Card`.
    pub(crate) fn signal(self) -> Signal {
        match self {
            Self::GeometryChanged => Signal::builder(self.name()).build(),
            Self::CardAdded | Self::CardRemoved | Self::CardVisibilityChanged => Signal::builder(self.name()).param_types([Card::static_type()]).build(),
        }
    }
}

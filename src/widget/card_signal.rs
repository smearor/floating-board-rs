use glib::subclass::Signal;

/// The signals `Card` emits, as a type-safe alternative to stringly-typed
/// signal names. Used by `signals()` in the widget implementation and by
/// every `emit_by_name` call site so each name lives in exactly one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CardSignal {
    /// Emitted after a drag or quad transform changed the position.
    Moved,
    /// Emitted on every rotation change (gesture or programmatic).
    Rotated,
    /// Emitted after a transform or `set_scale` changed the scale.
    Scaled,
    /// Emitted when the card is raised to the top.
    Raised,
    /// Emitted when the fold state toggles.
    Folded,
}

/// Every signal of the card, for `ObjectImpl::signals()`.
pub(crate) const CARD_SIGNALS: &[CardSignal] = &[
    CardSignal::Moved,
    CardSignal::Rotated,
    CardSignal::Scaled,
    CardSignal::Raised,
    CardSignal::Folded,
];

impl CardSignal {
    /// The static GTK signal name (`"card-moved"`).
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Moved => "card-moved",
            Self::Rotated => "card-rotated",
            Self::Scaled => "card-scaled",
            Self::Raised => "card-raised",
            Self::Folded => "card-folded",
        }
    }

    /// Builds the parameterless GTK signal declaration.
    pub(crate) fn signal(self) -> Signal {
        Signal::builder(self.name()).build()
    }
}

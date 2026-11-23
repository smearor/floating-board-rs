//! GTK widget layer: thin `ObjectSubclass` glue mapping gestures, layout and
//! transforms onto the pure domain operations in [`crate::domain`].

pub(crate) mod imp;

mod board;
mod board_signal;
mod card;
mod card_alignment;
mod card_close_action;
mod card_config;
mod card_layer;
mod card_layout;
mod card_satellites;
mod card_signal;
pub(crate) mod chrome;
mod circular_clip;
mod floating_board_layout;
mod satellite;
mod satellite_config;
#[cfg(feature = "fx")]
mod satellite_fx;
mod scaling;

pub(crate) mod css;

pub use board::FloatingBoard;
pub use card::Card;
pub use card_alignment::CardAlignment;
pub use card_close_action::CardCloseAction;
pub use card_config::CardConfig;
pub use card_layer::CardLayer;
pub use card_satellites::CardSatellites;
pub use satellite_config::SatelliteConfig;
pub use scaling::ScalingLayout;
pub use scaling::ScalingWidget;

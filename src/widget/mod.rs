//! GTK widget layer: thin `ObjectSubclass` glue mapping gestures, layout and
//! transforms onto the pure domain operations in [`crate::domain`].

pub(crate) mod imp;

mod board;
mod card;
mod card_config;
mod card_layer;
mod scaling;

pub(crate) mod css;

pub use board::FloatingBoard;
pub use card::Card;
pub use card_config::CardConfig;
pub use card_layer::CardLayer;
pub use scaling::ScalingLayout;
pub use scaling::ScalingWidget;

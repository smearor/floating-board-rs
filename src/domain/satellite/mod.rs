//! Satellite domain types: identity, layout configuration, rotation and
//! collision semantics, and the pure placement resolver (GTK-free).

mod collision_mode;
mod distribution;
mod id;
mod item;
mod kind;
mod layout;
mod placement;
mod radial_config;
mod resolver;
mod rotation_mode;

pub use collision_mode::SatelliteCollisionMode;
pub use distribution::SatelliteDistribution;
pub use id::SatelliteId;
pub use item::SatelliteItem;
pub use kind::SatelliteKind;
pub use layout::SatelliteLayout;
pub use placement::SatellitePlacement;
pub use radial_config::RadialConfig;
pub use resolver::resolve;
pub use rotation_mode::SatelliteRotationMode;

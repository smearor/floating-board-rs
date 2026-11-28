//! The `gauge` face - the cover dial plus its level satellites.

mod color;
mod draw;
mod satellite;

pub(crate) use draw::build_gauge;
pub(crate) use satellite::level_satellite;

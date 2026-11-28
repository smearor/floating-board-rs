//! The `compass` face - a compass dial: tick ring, cardinal letters and
//! a north needle. Draws its own disc - mounted as
//! `CardContent::Inscribed`, the surrounding ring stays card chrome
//! and the arc band reads like a header strip.

mod color;
mod draw;

pub(crate) use draw::build_compass;
pub(crate) use draw::paint_compass;

//! The table: background surface plus the seat roster and markers.

mod color;
mod draw;
mod seats;
mod users;

pub(crate) use draw::build_table_face;
pub(crate) use seats::MARKER_RADIUS;
pub(crate) use seats::marker_position;
pub(crate) use seats::seat_for_point;
pub(crate) use users::DemoUser;
pub(crate) use users::USERS;
pub(crate) use users::free_edge_offset;
pub(crate) use users::least_crowded_side;
pub(crate) use users::next_user;
pub(crate) use users::seat_badge_markup;

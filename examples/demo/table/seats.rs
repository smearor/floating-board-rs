//! Seat marker geometry on the table face.

use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Pixels;
use smearor_commons_geometry::Vec2;
use smearor_commons_layout::Seat;
use smearor_commons_layout::TableSide;

/// Radius of the user markers (also their hit target).
pub(crate) const MARKER_RADIUS: Pixels = Pixels::new(13.0);

/// Distance of the marker center from the table edge.
const MARKER_INSET: Pixels = Pixels::new(24.0);

/// Canvas position of a seat's marker: `offset` along the edge
/// (0 = left/top end), `MARKER_INSET` inside the area.
pub(crate) fn marker_position(seat: Seat, size: Vec2<Pixels>) -> Vec2<Pixels> {
    let offset = seat.offset.unwrap_or(Fraction::HALF).as_f64();
    let inset = MARKER_INSET.as_f64();
    match seat.side {
        TableSide::Bottom => Vec2::new(size.x * offset, size.y - inset),
        TableSide::Left => Vec2::new(inset, size.y * offset),
        TableSide::Top => Vec2::new(size.x * offset, inset),
        TableSide::Right => Vec2::new(size.x - inset, size.y * offset),
    }
}

/// The seat under a canvas point: the nearest edge becomes the side,
/// the position along it the pinned offset.
pub(crate) fn seat_for_point(point: Vec2<Pixels>, size: Vec2<Pixels>) -> Seat {
    let mut side = TableSide::Top;
    let mut nearest = point.y;
    for (candidate, distance) in [
        (TableSide::Bottom, size.y - point.y),
        (TableSide::Left, point.x),
        (TableSide::Right, size.x - point.x),
    ] {
        if distance < nearest {
            side = candidate;
            nearest = distance;
        }
    }
    let offset = match side {
        TableSide::Top | TableSide::Bottom => point.x / size.x,
        TableSide::Left | TableSide::Right => point.y / size.y,
    };
    Seat::at(side, Fraction::new(offset).clamped())
}

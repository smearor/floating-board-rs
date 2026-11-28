//! The table's seat roster: demo users, their letters and colors.

use smearor_commons_color::Color;
use smearor_commons_geometry::Fraction;
use smearor_commons_layout::Seat;
use smearor_commons_layout::TableSide;

/// The demo's seat roster: user letter and marker color per seat.
/// Passing a card hops between these occupied seats, skipping the
/// empty west edge.
/// One demo user: its seat (mutable), marker letter and badge color.
pub(crate) type DemoUser = (Seat, &'static str, Color<u8>);

pub(crate) const USERS: [DemoUser; 4] = [
    (Seat::new(TableSide::Bottom), USER_LETTERS[0], USER_COLORS[0]),
    (Seat::new(TableSide::Right), USER_LETTERS[1], USER_COLORS[1]),
    (Seat::at(TableSide::Top, Fraction::new(0.25)), USER_LETTERS[2], USER_COLORS[2]),
    (Seat::at(TableSide::Top, Fraction::new(0.75)), USER_LETTERS[3], USER_COLORS[3]),
];

/// Marker letters available to demo users.
const USER_LETTERS: [&str; 26] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z",
];

/// Marker colors, cycled by letter index.
const USER_COLORS: [Color<u8>; 10] = [
    Color::<u8>::new(0x35, 0x84, 0xe4),
    Color::<u8>::new(0x33, 0xd1, 0x7a),
    Color::<u8>::new(0xf6, 0xd3, 0x2d),
    Color::<u8>::new(0x91, 0x41, 0xac),
    Color::<u8>::new(0xe6, 0x61, 0x00),
    Color::<u8>::new(0xe0, 0x1b, 0x24),
    Color::<u8>::new(0x62, 0xa0, 0xea),
    Color::<u8>::new(0x57, 0xe3, 0x89),
    Color::<u8>::new(0xc0, 0x61, 0xcb),
    Color::<u8>::new(0x86, 0x5e, 0x3c),
];

/// First unused marker letter and its palette color.
pub(crate) fn next_user(users: &[DemoUser]) -> Option<(&'static str, Color<u8>)> {
    USER_LETTERS
        .iter()
        .enumerate()
        .find(|(_, letter)| !users.iter().any(|user| user.1 == **letter))
        .map(|(index, letter)| (*letter, USER_COLORS[index % USER_COLORS.len()]))
}

/// Table side holding the fewest users (ties keep `TableSide::ALL` order).
pub(crate) fn least_crowded_side(users: &[DemoUser]) -> TableSide {
    TableSide::ALL
        .into_iter()
        .min_by_key(|side| users.iter().filter(|user| user.0.side == *side).count())
        .unwrap_or(TableSide::Bottom)
}

/// An edge offset on `side` not already taken by another user, so a new
/// marker never spawns on top of an existing one.
pub(crate) fn free_edge_offset(users: &[DemoUser], side: TableSide) -> Fraction {
    const SPREAD: [f64; 9] = [0.5, 0.25, 0.75, 0.125, 0.375, 0.625, 0.875, 0.0625, 0.9375];
    let used: Vec<f64> = users
        .iter()
        .filter(|user| user.0.side == side)
        .map(|user| user.0.offset.unwrap_or(Fraction::HALF).as_f64())
        .collect();
    let offset = SPREAD
        .iter()
        .find(|candidate| !used.iter().any(|taken| (taken - *candidate).abs() < 0.02))
        .copied()
        .unwrap_or(((used.len() * 37) % 90 + 5) as f64 / 100.0);
    Fraction::new(offset)
}

/// Markup badge naming the user a seat belongs to (`-` unassigned).
pub(crate) fn seat_badge_markup(seat: Option<Seat>, users: &[DemoUser]) -> String {
    match users.iter().find(|(user_seat, _, _)| Some(*user_seat) == seat) {
        Some((_, letter, color)) => format!("<span background='{color}' foreground='white' weight='bold'> {letter} </span>"),
        None => "<span foreground='#888'> - </span>".to_string(),
    }
}

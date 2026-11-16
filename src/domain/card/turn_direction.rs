/// Direction a card is passed around the tabletop: which way along the
/// table edge the next user's seat lies.
///
/// Used by `FloatingBoard::rotate_to_next_user`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum TurnDirection {
    /// Pass to the seat one step clockwise around the table.
    #[default]
    Clockwise,
    /// Pass to the seat one step counter-clockwise around the table.
    CounterClockwise,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_direction_default_is_clockwise() {
        assert_eq!(TurnDirection::default(), TurnDirection::Clockwise);
    }
}

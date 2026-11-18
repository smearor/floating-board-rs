/// Where the pass-to-seat operations anchor a `Seat`'s table edge.
///
/// Configured per board via `FloatingBoard::set_seat_anchor` and
/// persisted in `BoardLayout`. The anchor affects only the seat
/// placement of `rotate_to_next_user`/`rotate_to_next_occupied_seat`
/// and the `Seat::nearest` derivation for unassigned cards - generic
/// `LayoutStrategy` runs keep operating in workspace coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum SeatAnchor {
    /// Seats sit at the screen edge - where the users physically
    /// sit. The whole placement (edge position, fan-out lane, margin,
    /// target rotation) is computed in widget space and mapped back
    /// through `to_workspace`, so a passed card lands at the visible
    /// edge facing its user even after the view was panned, zoomed or
    /// rotated. `Seat::nearest` derives from the rendered
    /// (card + view) rotation. The clamp domain widens to the shown
    /// region (`clamp_bounds`): cards may rest anywhere on screen,
    /// including seat positions beyond the workspace extent.
    #[default]
    View,
    /// Seats sit at the workspace extent - fixed positions on the
    /// virtual table, independent of the camera. Passing a card lands
    /// at the workspace edge wherever the view currently shows it,
    /// and `Seat::nearest` derives from the workspace rotation.
    Workspace,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seat_anchor_default_is_view() {
        assert_eq!(SeatAnchor::default(), SeatAnchor::View);
    }
}

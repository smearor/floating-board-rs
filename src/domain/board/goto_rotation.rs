/// How `FloatingBoard::goto_card` resolves the camera rotation at the
/// destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum GotoRotation {
    /// Rotate the camera so the card reads upright to the viewer
    /// (its top edge towards the top of the screen).
    #[default]
    Upright,
    /// Keep the current view rotation - the flight only pans and
    /// zooms, the card keeps its on-screen orientation.
    Keep,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goto_rotation_default_is_upright() {
        assert_eq!(GotoRotation::default(), GotoRotation::Upright);
    }
}

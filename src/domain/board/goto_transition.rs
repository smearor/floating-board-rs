/// How `FloatingBoard::goto_card` moves the camera to the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum GotoTransition {
    /// Apply the target view instantly.
    Snap,
    /// Fly straight there - pan, zoom and rotation glide together on
    /// the frame clock over `CardConfig::animation_speed_ms`.
    #[default]
    Glide,
    /// Prezi-style flight: the view first pulls back to a region
    /// covering both the current and the target view, then glides
    /// down onto the card. One `CardConfig::animation_speed_ms`
    /// flight over a waypoint, not two chained animations.
    Transit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn goto_transition_default_is_glide() {
        assert_eq!(GotoTransition::default(), GotoTransition::Glide);
    }
}

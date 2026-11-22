/// How a card's satellites react to overlaps with foreign cards or
/// satellites.
///
/// Same configurability pattern as `BoundsMode` (`Loose`/`Contained`):
/// strictness is opt-in per card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum SatelliteCollisionMode {
    /// Place blindly; the topmost element wins the pick (default).
    /// Consistent with cards freely overlapping each other.
    #[default]
    Ignore,
    /// Collision-aware: the resolver receives all card footprints and
    /// avoids overlaps - flip sidebar side, shift orbit start angle,
    /// shrink radius, hide colliding satellites.
    Avoid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_ignore() {
        assert_eq!(SatelliteCollisionMode::default(), SatelliteCollisionMode::Ignore);
    }
}

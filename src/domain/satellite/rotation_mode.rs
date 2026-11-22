/// How a card's satellites react to card rotation.
///
/// The mode controls two things independently: where a satellite sits
/// relative to the card (orbit angle) and how the satellite widget
/// itself is oriented on screen. For the `Sidebar` layout positions are
/// always card-local - the mode only decides widget orientation there;
/// `CounterRotate`'s orbit semantics are radial-layout matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum SatelliteRotationMode {
    /// Co-rotate (default): positions and widget rotation rigidly follow
    /// the card - the whole arrangement turns with the card.
    #[default]
    CoRotate,
    /// Positions follow the card rotation (they keep their angle
    /// relative to the card) but satellite widgets stay screen-upright.
    KeepCardAngle,
    /// Orbit positions stay fixed on screen (the orbit turns against
    /// the card) and widgets stay screen-upright. Sidebar placements
    /// treat this like [`Self::KeepCardAngle`]; the orbit semantics
    /// land with the radial layout.
    CounterRotate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_co_rotate() {
        assert_eq!(SatelliteRotationMode::default(), SatelliteRotationMode::CoRotate);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serializes_kebab_case() {
        assert_eq!(serde_json::to_string(&SatelliteRotationMode::KeepCardAngle).unwrap(), "\"keep-card-angle\"");
    }
}

use crate::domain::SatelliteCollisionMode;
use crate::domain::SatelliteLayout;
use crate::domain::SatelliteRotationMode;
use crate::widget::satellite_config::SatelliteConfig;

/// Per-card satellite configuration, stored in `CardConfig::satellites`.
///
/// Carries the layout and interaction semantics for all of the card's
/// satellites - both the explicit `satellites` list and chrome elements
/// placed via `ChromeSlot::satellite(..)`, which join the same set.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CardSatellites {
    /// Layout strategy for this card's satellites.
    #[cfg_attr(feature = "serde", serde(default))]
    pub layout: SatelliteLayout,
    /// Rotation semantics vs. card rotation.
    #[cfg_attr(feature = "serde", serde(default))]
    pub rotation_mode: SatelliteRotationMode,
    /// Collision handling vs. foreign cards and satellites.
    #[cfg_attr(feature = "serde", serde(default))]
    pub collision: SatelliteCollisionMode,
    /// Configured satellites, in dock/orbit order. Chrome satellites
    /// come either from this list (`SatelliteKind::Chrome`) or from
    /// `ChromeSlot::satellite` entries in `CardConfig::chrome`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub satellites: Vec<SatelliteConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip_without_widget() {
        let satellites = CardSatellites::default();
        let json = serde_json::to_string(&satellites).unwrap();
        let back: CardSatellites = serde_json::from_str(&json).unwrap();
        assert_eq!(back.satellites.len(), 0);
    }
}

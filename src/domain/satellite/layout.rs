use crate::domain::RadialConfig;
use smearor_commons_layout::SideSet;

/// Arrangement of a card's satellites.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(rename_all = "kebab-case"))]
pub enum SatelliteLayout {
    /// Satellites docked as a strip along one to four card edges
    /// (bottom, left, top, right), relative to the card's local frame.
    Sidebar {
        /// The edges the strip docks to - one to four, non-empty by
        /// `SideSet`'s invariant.
        sides: SideSet,
    },
    /// Satellites on an orbit around the card (see [`RadialConfig`]).
    Radial(RadialConfig),
}

impl Default for SatelliteLayout {
    /// A single sidebar strip docked below the card.
    fn default() -> Self {
        Self::Sidebar { sides: SideSet::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smearor_commons_layout::TableSide;

    #[cfg(feature = "serde")]
    #[test]
    fn serde_round_trip() {
        let layout = SatelliteLayout::Sidebar {
            sides: SideSet::try_from_sides([TableSide::Bottom, TableSide::Right]).expect("non-empty"),
        };
        let json = serde_json::to_string(&layout).expect("serialize");
        let parsed: SatelliteLayout = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, layout);
    }
}

use crate::domain::CardGeometry;
use crate::domain::SatelliteId;

/// Resolved placement of one satellite in workspace coordinates -
/// the resolver's answer for [`crate::domain::SatelliteItem`], applied
/// to the satellite's transform stack exactly like a card geometry.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SatellitePlacement {
    /// Which satellite this placement answers.
    pub id: SatelliteId,
    /// Target transform - same unit as [`CardGeometry`]
    /// (position + rotation + scale).
    pub geometry: CardGeometry,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CardPosition;
    use smearor_commons_geometry::Pixels;
    use smearor_commons_geometry::Vec2;

    #[test]
    fn placement_carries_id_and_geometry() {
        let placement = SatellitePlacement {
            id: SatelliteId::new("fold"),
            geometry: CardGeometry {
                position: CardPosition::new(Vec2::<Pixels>::new(10.0, 20.0)),
                ..Default::default()
            },
        };
        assert_eq!(placement.id.as_str(), "fold");
    }
}

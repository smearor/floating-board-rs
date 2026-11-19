use crate::domain::CardGeometry;
use crate::domain::CardId;
use smearor_commons_layout::Seat;

/// One entry of a [`crate::domain::BoardLayout`]: card id plus its geometry.
///
/// Serialized flattened, so a layout file reads:
///
/// ```toml
/// [[cards]]
/// id = "controls"
/// position = [480.0, 660.0]
/// rotation = 180.0
/// scale = 1.0
/// folded = false
/// ```
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CardPlacement {
    /// Stable card identifier.
    pub id: CardId,
    /// Geometric state of the card.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub geometry: CardGeometry,
    /// Whether the card is folded to its header strip. Defaults to
    /// unfolded when reading layouts written before the field existed.
    #[cfg_attr(feature = "serde", serde(default))]
    pub folded: bool,
    /// Seat the card was assigned to when the snapshot was taken.
    /// Defaults to unassigned when reading layouts written before the
    /// field existed.
    #[cfg_attr(feature = "serde", serde(default))]
    pub seat: Option<Seat>,
    /// Whether the card is pinned to the screen (`CardConfig::sticky`).
    /// Sticky geometry is stored in widget pixels, not workspace
    /// coordinates. Defaults to `false` when reading layouts written
    /// before the field existed.
    #[cfg_attr(feature = "serde", serde(default))]
    pub sticky: bool,
}

impl CardPlacement {
    /// Creates a placement from id, geometry and folded state.
    pub fn new(id: CardId, geometry: CardGeometry, folded: bool) -> Self {
        Self {
            id,
            geometry,
            folded,
            seat: None,
            sticky: false,
        }
    }

    /// Returns the placement with the given seat assignment.
    pub fn with_seat(self, seat: Option<Seat>) -> Self {
        Self { seat, ..self }
    }

    /// Returns the placement with the given sticky flag.
    pub fn with_sticky(self, sticky: bool) -> Self {
        Self { sticky, ..self }
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;
    use crate::domain::CardPosition;
    use crate::domain::ScaleFactor;
    use smearor_commons_geometry::Rotation;
    use smearor_commons_geometry::Vec2;

    #[test]
    fn placement_serializes_flat() {
        let placement = CardPlacement::new(
            CardId::new("controls"),
            CardGeometry {
                position: CardPosition::from(Vec2::new(480.0, 660.0)),
                rotation: Rotation::from_degrees(180.0),
                scale: ScaleFactor::new(1.0),
            },
            false,
        );
        let json = serde_json::to_value(&placement).unwrap();
        assert_eq!(json["id"], serde_json::json!("controls"));
        assert_eq!(json["position"], serde_json::json!([480.0, 660.0]));
        assert_eq!(json["rotation"], serde_json::json!(180.0));
        assert!(json.get("geometry").is_none());
    }
}

use crate::domain::CardGeometry;
use crate::domain::CardId;

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
}

impl CardPlacement {
    /// Creates a placement from id, geometry and folded state.
    pub fn new(id: CardId, geometry: CardGeometry, folded: bool) -> Self {
        Self { id, geometry, folded }
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;
    use crate::domain::CardPosition;
    use crate::domain::ScaleFactor;
    use smearor_wrot_rotation::SmearorRotation;

    #[test]
    fn placement_serializes_flat() {
        let placement = CardPlacement::new(
            CardId::new("controls"),
            CardGeometry {
                position: CardPosition::new(480.0, 660.0),
                rotation: SmearorRotation::Deg180,
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

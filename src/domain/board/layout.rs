use crate::domain::CardPlacement;

/// Serializable snapshot of every card's geometry on the board.
///
/// Persistence is consumer-side: `FloatingBoard::layout_snapshot` produces this
/// value and `FloatingBoard::restore_layout` consumes it; the application
/// decides where and in which format it is stored (e.g. `gui-cards.toml`).
/// `version` is reserved for future migrations.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BoardLayout {
    /// Layout format version; currently `1`.
    pub version: u8,
    /// Per-card placements in stable (id-sorted) order.
    pub cards: Vec<CardPlacement>,
}

/// Current layout format version.
pub const LAYOUT_VERSION: u8 = 1;

impl BoardLayout {
    /// Creates an empty layout at the current version.
    pub fn empty() -> Self {
        Self {
            version: LAYOUT_VERSION,
            cards: Vec::new(),
        }
    }

    /// Creates a layout from placements at the current version.
    pub fn new(cards: Vec<CardPlacement>) -> Self {
        Self {
            version: LAYOUT_VERSION,
            cards,
        }
    }

    /// Returns whether the layout contains no cards.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Number of card placements in the layout.
    pub fn len(&self) -> usize {
        self.cards.len()
    }
}

impl Default for BoardLayout {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(all(test, feature = "serde"))]
mod tests {
    use super::*;
    use crate::domain::CardGeometry;
    use crate::domain::CardId;
    use crate::domain::CardPosition;
    use crate::domain::ScaleFactor;
    use smearor_commons_geometry::Vec2;
    use smearor_wrot_rotation::SmearorRotation;

    #[test]
    fn layout_toml_roundtrip_matches_concept_format() {
        let source = r#"
version = 1

[[cards]]
id = "controls"
position = [480.0, 660.0]
rotation = 180.0
scale = 1.0
"#;
        let layout: BoardLayout = toml::from_str(source).unwrap();
        assert_eq!(layout.version, 1);
        assert_eq!(layout.len(), 1);
        let placement = &layout.cards[0];
        assert_eq!(placement.id, CardId::new("controls"));
        assert_eq!(placement.geometry.position, CardPosition::from(Vec2::new(480.0, 660.0)));
        assert_eq!(placement.geometry.rotation.to_degrees(), 180.0);
        assert_eq!(placement.geometry.scale, ScaleFactor::new(1.0));

        let serialized = toml::to_string_pretty(&layout).unwrap();
        let reparsed: BoardLayout = toml::from_str(&serialized).unwrap();
        assert_eq!(reparsed.len(), layout.len());
        assert_eq!(reparsed.cards[0].geometry.rotation.to_degrees(), 180.0);
    }

    #[test]
    fn layout_json_roundtrip() {
        let layout = BoardLayout::new(vec![CardPlacement::new(
            CardId::new("a"),
            CardGeometry {
                position: CardPosition::from(Vec2::new(1.0, 2.0)),
                rotation: SmearorRotation::DEG_90,
                scale: ScaleFactor::new(1.5),
            },
            true,
        )]);
        let parsed: BoardLayout = serde_json::from_str(&serde_json::to_string(&layout).unwrap()).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed.cards[0].geometry.scale, ScaleFactor::new(1.5));
    }
}

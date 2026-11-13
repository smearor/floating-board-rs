use crate::domain::CardPosition;
use crate::domain::ScaleFactor;
use smearor_commons_geometry::Rotation;

/// Complete geometric state of a card: center position, rotation and scale.
///
/// This is the unit of persistence: a `CardGeometry` per card ends up in
/// [`crate::domain::BoardLayout`] snapshots.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CardGeometry {
    /// Center point of the card in board coordinates.
    pub position: CardPosition,
    /// Rotation of the card; serialized as degrees (`rotation = 180.0`).
    #[cfg_attr(feature = "serde", serde(with = "rotation_degrees"))]
    pub rotation: Rotation,
    /// Scale factor of the card (`1.0` = natural size).
    pub scale: ScaleFactor,
}

impl Default for CardGeometry {
    fn default() -> Self {
        Self {
            position: CardPosition::default(),
            rotation: Rotation::ZERO,
            scale: ScaleFactor::default(),
        }
    }
}

#[cfg(feature = "serde")]
mod rotation_degrees {
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;
    use smearor_commons_geometry::Rotation;

    /// Serializes the rotation as a plain degree value (`180.0`), matching the
    /// persisted layout format - independent of `Rotation`'s own serde
    /// representation.
    pub fn serialize<S: Serializer>(rotation: &Rotation, serializer: S) -> Result<S::Ok, S::Error> {
        rotation.degrees().serialize(serializer)
    }

    /// Deserializes a degree value into [`Rotation`].
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Rotation, D::Error> {
        Ok(Rotation::from_degrees(f64::deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smearor_commons_geometry::Vec2;

    #[test]
    fn geometry_default() {
        let geometry = CardGeometry::default();
        assert_eq!(geometry.rotation.degrees(), 0.0);
        assert_eq!(geometry.scale.value(), 1.0);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn geometry_serializes_rotation_as_degrees() {
        let geometry = CardGeometry {
            position: CardPosition::from(Vec2::new(480.0, 660.0)),
            rotation: Rotation::from_degrees(180.0),
            scale: ScaleFactor::new(1.0),
        };
        let json = serde_json::to_value(geometry).unwrap();
        assert_eq!(json["position"], serde_json::json!([480.0, 660.0]));
        assert_eq!(json["rotation"], serde_json::json!(180.0));
        assert_eq!(json["scale"], serde_json::json!(1.0));

        let parsed: CardGeometry = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.rotation.degrees(), 180.0);
    }
}

use crate::domain::CardPosition;
use crate::domain::ScaleFactor;
use smearor_commons_animation::Interpolate;
use smearor_commons_geometry::Fraction;
use smearor_commons_geometry::Rotation;

/// Complete geometric state of a card: center position, rotation and scale.
///
/// This is the unit of persistence: a `CardGeometry` per card ends up in
/// [`crate::domain::BoardLayout`] snapshots.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CardGeometry {
    /// Center point of the card in workspace coordinates.
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

/// Component-wise interpolation on one clock: the position slides
/// along `Vec2<Pixels>`, the rotation follows the shortest arc via
/// `RotationRad` (re-quantized so snapped targets stay `QuarterTurn`)
/// and the scale blends linearly - the animation unit of layout
/// transitions.
impl Interpolate for CardGeometry {
    fn interpolate(self, target: Self, t: Fraction) -> Self {
        Self {
            position: CardPosition::from(self.position.0.interpolate(target.position.0, t)),
            rotation: Rotation::quantized(self.rotation.to_rotation_rad().interpolate(target.rotation.to_rotation_rad(), t)),
            scale: self.scale.interpolate(target.scale, t),
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
    use smearor_commons_geometry::QuarterTurn;
    use smearor_commons_geometry::Vec2;

    /// Interpolation endpoints map `from`/`to` exactly.
    #[test]
    fn interpolate_endpoints_are_exact() {
        let from = CardGeometry {
            position: CardPosition::from(Vec2::new(10.0, 20.0)),
            rotation: Rotation::from_degrees(30.0),
            scale: ScaleFactor::new(0.5),
        };
        let to = CardGeometry {
            position: CardPosition::from(Vec2::new(110.0, 220.0)),
            rotation: Rotation::from_degrees(120.0),
            scale: ScaleFactor::new(2.0),
        };
        let start = from.interpolate(to, Fraction::new(0.0));
        assert_eq!(start.position.0, from.position.0);
        assert_eq!(start.rotation.degrees(), from.rotation.degrees());
        assert_eq!(start.scale.value(), from.scale.value());
        let end = from.interpolate(to, Fraction::new(1.0));
        assert_eq!(end.position.0, to.position.0);
        assert_eq!(end.rotation.degrees(), to.rotation.degrees());
        assert_eq!(end.scale.value(), to.scale.value());
    }

    /// The midpoint blends all three components.
    #[test]
    fn interpolate_midpoint_blends_all_components() {
        let from = CardGeometry {
            position: CardPosition::from(Vec2::new(0.0, 0.0)),
            rotation: Rotation::from_degrees(0.0),
            scale: ScaleFactor::new(1.0),
        };
        let to = CardGeometry {
            position: CardPosition::from(Vec2::new(100.0, 200.0)),
            rotation: Rotation::from_degrees(90.0),
            scale: ScaleFactor::new(3.0),
        };
        let mid = from.interpolate(to, Fraction::new(0.5));
        assert_eq!(mid.position.0, Vec2::new(50.0, 100.0));
        assert_eq!(mid.rotation.degrees(), 45.0);
        assert_eq!(mid.scale.value(), 2.0);
    }

    /// Rotations take the shortest arc: 350° -> 10° passes through 0°,
    /// not backwards through 180°.
    #[test]
    fn interpolate_rotation_takes_the_shortest_arc() {
        let from = CardGeometry {
            rotation: Rotation::from_degrees(350.0),
            ..CardGeometry::default()
        };
        let to = CardGeometry {
            rotation: Rotation::from_degrees(10.0),
            ..CardGeometry::default()
        };
        let mid = from.interpolate(to, Fraction::new(0.5));
        assert_eq!(mid.rotation.degrees(), 0.0);
    }

    /// An interpolated exact quarter turn re-quantizes to `QuarterTurn`.
    #[test]
    fn interpolate_requantizes_exact_quarter_turns() {
        let from = CardGeometry {
            rotation: Rotation::from_degrees(0.0),
            ..CardGeometry::default()
        };
        let to = CardGeometry {
            rotation: Rotation::from_degrees(90.0),
            ..CardGeometry::default()
        };
        assert_eq!(from.interpolate(to, Fraction::new(1.0)).rotation, Rotation::QuarterTurn(QuarterTurn::Deg90));
    }

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

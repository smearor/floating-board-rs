/// A rotation stored in radians.
///
/// Newtype over `f64` so a raw number can no longer pass as a rotation by
/// accident; angles arriving in degrees convert through
/// [`RotationRad::from_degrees`].
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RotationRad(pub f64);

impl RotationRad {
    /// Creates a rotation from radians.
    pub fn new(radians: f64) -> Self {
        Self(radians)
    }

    /// Creates a rotation from degrees.
    pub fn from_degrees(degrees: f64) -> Self {
        Self(degrees.to_radians())
    }

    /// The rotation in radians.
    pub fn radians(&self) -> f64 {
        self.0
    }

    /// The rotation in degrees.
    pub fn to_degrees(&self) -> f64 {
        self.0.to_degrees()
    }
}

impl From<f64> for RotationRad {
    fn from(radians: f64) -> Self {
        Self(radians)
    }
}

impl From<RotationRad> for f64 {
    fn from(rotation: RotationRad) -> Self {
        rotation.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn degrees_roundtrip() {
        let rotation = RotationRad::from_degrees(90.0);
        assert!((rotation.radians() - std::f64::consts::FRAC_PI_2).abs() < f64::EPSILON);
        assert!((rotation.to_degrees() - 90.0).abs() < f64::EPSILON);
    }
}

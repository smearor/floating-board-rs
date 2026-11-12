use crate::domain::ScaleBounds;
use smearor_commons_geometry::ScaleFactor as GeometryScaleFactor;

/// Scale factor of a card (`1.0` = natural size).
///
/// The factor is always strictly positive; clamping against per-card limits
/// happens via [`ScaleBounds`]. Wraps the technical
/// [`GeometryScaleFactor`] primitive with card-domain validation.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(transparent))]
pub struct ScaleFactor(GeometryScaleFactor);

/// Smallest positive scale to avoid degenerate zero-sized cards.
const MIN_SCALE: f64 = 0.01;

impl ScaleFactor {
    /// Creates a scale factor, clamped to a strictly positive value.
    pub fn new(value: f64) -> Self {
        if value.is_finite() {
            Self(GeometryScaleFactor::new(value.max(MIN_SCALE)))
        } else {
            Self::default()
        }
    }

    /// Returns the raw factor (`1.0` = natural size).
    pub fn value(&self) -> f64 {
        self.0.as_f64()
    }

    /// Returns the factor clamped into the given bounds.
    pub fn clamped(&self, bounds: &ScaleBounds) -> Self {
        Self::new(bounds.clamp(self.value()))
    }
}

impl Default for ScaleFactor {
    fn default() -> Self {
        Self(GeometryScaleFactor::ONE)
    }
}

impl From<f64> for ScaleFactor {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<ScaleFactor> for f64 {
    fn from(value: ScaleFactor) -> Self {
        value.0.as_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_factor_defaults_to_one() {
        assert_eq!(ScaleFactor::default().value(), 1.0);
    }

    #[test]
    fn scale_factor_never_zero_or_negative() {
        assert!(ScaleFactor::new(0.0).value() > 0.0);
        assert!(ScaleFactor::new(-2.0).value() > 0.0);
        assert_eq!(ScaleFactor::new(f64::NAN).value(), 1.0);
    }

    #[test]
    fn scale_factor_clamped_to_bounds() {
        let bounds = ScaleBounds::new(0.5, 2.0);
        assert_eq!(ScaleFactor::new(4.0).clamped(&bounds).value(), 2.0);
        assert_eq!(ScaleFactor::new(0.1).clamped(&bounds).value(), 0.5);
    }
}

/// Inclusive `[min, max]` limits for a card's scale factor.
///
/// Used by `CardConfig::scale_limits`; defaults to `0.5 - 2.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ScaleBounds {
    /// Smallest allowed scale factor.
    pub min: f64,
    /// Largest allowed scale factor.
    pub max: f64,
}

impl ScaleBounds {
    /// Creates bounds; the arguments are normalized so `min <= max` and both
    /// are strictly positive.
    pub fn new(min: f64, max: f64) -> Self {
        let min = min.max(f64::EPSILON);
        let max = max.max(f64::EPSILON);
        if min <= max { Self { min, max } } else { Self { min: max, max: min } }
    }

    /// Clamps a value into these bounds.
    pub fn clamp(&self, value: f64) -> f64 {
        value.clamp(self.min, self.max)
    }

    /// Returns whether the value lies within the bounds.
    pub fn contains(&self, value: f64) -> bool {
        value >= self.min && value <= self.max
    }
}

impl Default for ScaleBounds {
    fn default() -> Self {
        Self { min: 0.5, max: 2.0 }
    }
}

impl From<(f64, f64)> for ScaleBounds {
    fn from(value: (f64, f64)) -> Self {
        Self::new(value.0, value.1)
    }
}

impl From<ScaleBounds> for (f64, f64) {
    fn from(value: ScaleBounds) -> Self {
        (value.min, value.max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_normalize_order() {
        let bounds = ScaleBounds::new(2.0, 0.5);
        assert_eq!(bounds.min, 0.5);
        assert_eq!(bounds.max, 2.0);
    }

    #[test]
    fn bounds_clamp_values() {
        let bounds = ScaleBounds::default();
        assert_eq!(bounds.clamp(0.1), 0.5);
        assert_eq!(bounds.clamp(1.0), 1.0);
        assert_eq!(bounds.clamp(9.0), 2.0);
    }

    #[test]
    fn bounds_from_tuple() {
        let bounds: ScaleBounds = (0.25, 4.0).into();
        assert!(bounds.contains(3.0));
        assert!(!bounds.contains(5.0));
    }
}

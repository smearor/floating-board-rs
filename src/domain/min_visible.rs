/// Minimum overlap in pixels a card must keep inside the board bounds.
///
/// Boundary clamping pushes a card back until at least this much of its
/// footprint remains visible; defaults to `40.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(transparent))]
pub struct MinVisible(pub(crate) f64);

impl MinVisible {
    /// Creates the threshold; negative values are clamped to zero.
    pub fn new(value: f64) -> Self {
        if value.is_finite() { Self(value.max(0.0)) } else { Self::default() }
    }

    /// Returns the threshold in pixels.
    pub fn value(&self) -> f64 {
        self.0
    }
}

impl Default for MinVisible {
    fn default() -> Self {
        Self(40.0)
    }
}

impl From<f64> for MinVisible {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<MinVisible> for f64 {
    fn from(value: MinVisible) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_visible_defaults() {
        assert_eq!(MinVisible::default().value(), 40.0);
    }

    #[test]
    fn min_visible_clamps_negative() {
        assert_eq!(MinVisible::new(-10.0).value(), 0.0);
    }
}

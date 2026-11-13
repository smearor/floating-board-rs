use smearor_commons_geometry::Pixels;

/// Minimum overlap in pixels a card must keep inside the board bounds.
///
/// Boundary clamping pushes a card back until at least this much of its
/// footprint remains visible; defaults to `40.0` px.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(transparent))]
pub struct MinVisible(pub(crate) Pixels);

impl MinVisible {
    /// Creates the threshold; negative values are clamped to zero.
    pub fn new(value: impl Into<Pixels>) -> Self {
        let value = value.into();
        if value.as_f64().is_finite() {
            Self(Pixels::new(value.as_f64().max(0.0)))
        } else {
            Self::default()
        }
    }

    /// Returns the threshold in pixels.
    pub fn value(&self) -> Pixels {
        self.0
    }
}

impl Default for MinVisible {
    fn default() -> Self {
        Self(Pixels::new(40.0))
    }
}

impl From<f64> for MinVisible {
    fn from(value: f64) -> Self {
        Self::new(value)
    }
}

impl From<MinVisible> for f64 {
    fn from(value: MinVisible) -> Self {
        value.0.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_visible_defaults() {
        assert_eq!(MinVisible::default().value(), Pixels::new(40.0));
    }

    #[test]
    fn min_visible_clamps_negative() {
        assert_eq!(MinVisible::new(-10.0).value(), Pixels::new(0.0));
    }
}

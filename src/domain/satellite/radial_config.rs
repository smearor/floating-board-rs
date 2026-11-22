use crate::domain::SatelliteDistribution;
use smearor_commons_geometry::Arc;
use smearor_commons_geometry::Distance;
use smearor_commons_geometry::Pixels;

/// Parameters of the `Radial` satellite layout: satellites sit on an
/// orbit around the card - a full circle or a covered arc, evenly
/// distributed or pinned to anchor angles.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RadialConfig {
    /// Covered arc: full circle (`Arc::FULL`) or a partial arc
    /// (start direction + angular width), in card-local space.
    pub span: Arc,
    /// How satellites distribute along `span`.
    pub distribution: SatelliteDistribution,
    /// Orbit radius measured outward from the card's outline.
    pub radius: Distance<Pixels>,
}

impl Default for RadialConfig {
    fn default() -> Self {
        Self {
            span: Arc::FULL,
            distribution: SatelliteDistribution::default(),
            radius: Distance::new(48.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_full_circle_even() {
        let config = RadialConfig::default();
        assert_eq!(config.span, Arc::FULL);
        assert_eq!(config.distribution, SatelliteDistribution::Even);
    }
}
